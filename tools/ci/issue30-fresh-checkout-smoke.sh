#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
run_id="${GITHUB_RUN_ID:-local}-$$"
smoke_root="${ISSUE30_SMOKE_ROOT:-$repo_root/target/issue30-fresh-checkout-smoke-$run_id}"
state_dir="$smoke_root/state"
mkdir -p "$state_dir"

cargo_cmd=(cargo)
if [[ -n "${ISSUE30_CARGO_TOOLCHAIN:-}" ]]; then
  cargo_cmd=("${cargo_cmd[@]}" "+${ISSUE30_CARGO_TOOLCHAIN}")
fi

trace_path="$state_dir/issue30-trace.jsonl"
memory_governance_trace_path="$state_dir/issue30-memory-governance-trace.jsonl"
memory_path="$state_dir/memory.ndkv"
experience_path="$state_dir/experience.ndkv"
adaptive_path="$state_dir/adaptive.ndkv"
roundtrip_stdout="$smoke_root/roundtrip.stdout"
memory_governance_stdout="$smoke_root/memory-governance-benchmark.stdout"

roundtrip_args=(
  run --locked --package rust-norion --
  --benchmark-roundtrip
  --inspect-state
  --inspect-gate
  --trace "$trace_path"
  --trace-schema-gate "$trace_path"
  --memory "$memory_path"
  --experience "$experience_path"
  --adaptive "$adaptive_path"
  --profile coding
  --runtime-kv-exchange
  --runtime-layers 6
  --runtime-hidden-size 64
  --runtime-attention-heads 4
  --runtime-kv-heads 2
  --runtime-local-window 32
  --inspect-min-runtime-kv-memories 1
  --inspect-min-experiences 1
  --inspect-min-runtime-model-experiences 1
  --inspect-min-runtime-adapter-experiences 1
  --inspect-max-runtime-adapter-selection-mismatches 0
  --inspect-min-runtime-forward-energy-experiences 1
  --inspect-min-runtime-kv-influence-experiences 1
  --inspect-min-runtime-kv-precision-experiences 1
  --inspect-max-runtime-kv-precision-mismatches 0
  --inspect-min-runtime-device-execution-experiences 1
  --inspect-min-runtime-kv-import-experiences 1
  --inspect-min-runtime-kv-export-experiences 1
  --inspect-min-live-memory-feedback-experiences 1
  --inspect-min-live-memory-feedback-updates 1
  --inspect-require-runtime-kv-dimensions
)

"${cargo_cmd[@]}" "${roundtrip_args[@]}" >"$roundtrip_stdout"

memory_governance_args=(
  run --locked --package rust-norion --
  --benchmark "$memory_governance_trace_path"
  --memory "$memory_path"
  --experience "$experience_path"
  --adaptive "$adaptive_path"
  --profile coding
  --runtime-kv-exchange
  --runtime-layers 6
  --runtime-hidden-size 64
  --runtime-attention-heads 4
  --runtime-kv-heads 2
  --runtime-local-window 32
  --retention-stale-after 1
  --retention-decay-rate 0.50
  --retention-remove-below 0.15
  --retention-remove-after-failures 1
  --compaction-threshold 0.90
  --compaction-max-candidates 256
  --compaction-max-merges 2
  --benchmark-min-memory-retention-activity-cases 1
  --benchmark-min-memory-compaction-activity-cases 1
)

"${cargo_cmd[@]}" "${memory_governance_args[@]}" >"$memory_governance_stdout"

disk_kv_compact_reopen_stdout="$smoke_root/disk-kv-compact-reopen.stdout"
memory_admission_ledger_reopen_stdout="$smoke_root/memory-admission-ledger-reopen.stdout"
memory_admission_authorized_fixture_apply_stdout="$smoke_root/memory-admission-authorized-fixture-apply.stdout"
memory_admission_runtime_preview_apply_stdout="$smoke_root/memory-admission-runtime-preview-apply.stdout"
memory_admission_read_only_authorized_append_stdout="$smoke_root/memory-admission-read-only-authorized-append.stdout"
memory_admission_review_scope_required_stdout="$smoke_root/memory-admission-review-scope-required.stdout"
memory_admission_invalid_shape_rejection_stdout="$smoke_root/memory-admission-invalid-shape-rejection.stdout"
reasoning_chaperone_fold_guard_stdout="$smoke_root/reasoning-chaperone-fold-guard.stdout"
"${cargo_cmd[@]}" test --locked --package rust-norion compact_keeps_latest_values >"$disk_kv_compact_reopen_stdout"
"${cargo_cmd[@]}" test --locked --package rust-norion writer_gate_append_is_idempotent_after_store_reopen >"$memory_admission_ledger_reopen_stdout"
"${cargo_cmd[@]}" test --locked --package rust-norion writer_gate_rehydrates_applied_authorized_records_from_existing_ledger >"$memory_admission_authorized_fixture_apply_stdout"
"${cargo_cmd[@]}" test --locked --package rust-norion runtime_memory_admission_preview_applies_after_approved_writer_policy >"$memory_admission_runtime_preview_apply_stdout"
"${cargo_cmd[@]}" test --locked --package rust-norion writer_gate_refuses_authorized_append_on_read_only_store >"$memory_admission_read_only_authorized_append_stdout"
"${cargo_cmd[@]}" test --locked --package rust-norion gene_segment_kv_writer_gate_rejects_missing_review_scope_digests >"$memory_admission_review_scope_required_stdout"
"${cargo_cmd[@]}" test --locked --package rust-norion gene_segment_kv_records_reject_invalid_shape_without_write >"$memory_admission_invalid_shape_rejection_stdout"
"${cargo_cmd[@]}" test --locked --package norion-agent reasoning_chaperone >"$reasoning_chaperone_fold_guard_stdout"
disk_kv_compact_reopen_verified=true
disk_kv_compact_reopen_test='disk_kv::tests::compact_keeps_latest_values'
memory_admission_ledger_reopen_verified=true
memory_admission_ledger_reopen_test='memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen'
memory_admission_authorized_fixture_apply_verified=true
memory_admission_authorized_fixture_apply_test='memory_admission::tests::writer_gate_rehydrates_applied_authorized_records_from_existing_ledger'
memory_admission_authorized_fixture_authorized=1
memory_admission_authorized_fixture_applied=1
memory_admission_authorized_fixture_admitted=1
memory_admission_authorized_fixture_rehydrated=1
memory_admission_authorized_fixture_reopened_records=1
memory_admission_authorized_fixture_ledger_bytes_nonzero=true
memory_admission_runtime_preview_apply_verified=true
memory_admission_runtime_preview_apply_test='tests::benchmark_state::runtime_memory_admission_preview_applies_after_approved_writer_policy'
memory_admission_runtime_preview_authorized=10
memory_admission_runtime_preview_applied=10
memory_admission_runtime_preview_admitted=10
memory_admission_runtime_preview_rehydrated=10
memory_admission_read_only_authorized_append_denied=true
memory_admission_read_only_authorized_append_test='memory_admission::tests::writer_gate_refuses_authorized_append_on_read_only_store'
memory_admission_read_only_authorized_append_preserved_existing_bytes=true
memory_admission_review_scope_required_verified=true
memory_admission_review_scope_required_test='memory_admission::tests::gene_segment_kv_writer_gate_rejects_missing_review_scope_digests'
memory_admission_review_scope_required_tenant_rejection='review_packet_tenant_scope_digest_missing'
memory_admission_review_scope_required_session_rejection='review_packet_session_scope_digest_missing'
memory_admission_review_scope_required_authorized=0
memory_admission_review_scope_required_appended=0
memory_admission_invalid_shape_rejection_verified=true
memory_admission_invalid_shape_rejection_test='memory_admission::tests::gene_segment_kv_records_reject_invalid_shape_without_write'
memory_admission_invalid_shape_source_hash_present=false
memory_admission_invalid_shape_kv_shape_valid=false
memory_admission_invalid_shape_ledger_rejected=1
memory_admission_invalid_shape_ledger_authorized=0
memory_admission_invalid_shape_preview_read_only=true
memory_admission_invalid_shape_preview_write_allowed=false
memory_autophagy_context_pressure_score=115
memory_autophagy_retrieval_noise_score=10
memory_autophagy_stale_decay_candidates=1
memory_autophagy_duplicate_merge_candidates=1
memory_autophagy_gist_recomposition_candidates=2
memory_autophagy_active_recall_prune_candidates=5
memory_autophagy_quarantine_candidates=3
memory_autophagy_live_delete_allowed=false
memory_autophagy_durable_mutation_allowed=false
memory_autophagy_reason_codes='active_recall_prune_preview|gist_recomposition_preview|quarantine_preview|recycle_preview'
issue503_chaperone_fold_guard_verified=true
issue503_chaperone_fold_guard_test='chaperone::tests::reasoning_chaperone'
issue503_fold_status=repair
issue503_undefined_capability_count=1
issue503_contradiction_count=1
issue503_ungated_side_effect_count=1
issue503_missing_evidence_count=1
issue503_repair_task_count=1
issue503_raw_cot_captured=false
issue503_raw_prompt_captured=false

roundtrip_proof="$smoke_root/roundtrip-proof.txt"
trace_report="$smoke_root/trace-report.txt"
state_gate="$smoke_root/state-gate.txt"
grep -m1 '^persistent_roundtrip:' "$roundtrip_stdout" >"$roundtrip_proof"
grep -m1 '^state_inspection_gate:' "$roundtrip_stdout" >"$state_gate"

field_value() {
  local line="$1"
  local key="$2"
  printf '%s\n' "$line" | tr ' ' '\n' | sed -n "s/^${key}=//p" | head -n 1
}

require_nonempty() {
  local source="$1"
  local key="$2"
  local value="$3"
  if [[ -z "$value" ]]; then
    echo "$source missing $key" >&2
    exit 1
  fi
}

trace_summary_line="$(grep -m1 '^trace_schema_gate:' "$roundtrip_stdout" || true)"
if [[ -z "$trace_summary_line" ]]; then
  echo "roundtrip stdout missing trace_schema_gate summary" >&2
  exit 1
fi
benchmark_summary_line="$(grep -m1 '^cases=' "$memory_governance_stdout" || true)"
if [[ -z "$benchmark_summary_line" ]]; then
  echo "memory governance benchmark stdout missing summary" >&2
  exit 1
fi
python_cmd=()
if [[ -n "${PYTHON:-}" ]] && "$PYTHON" --version >/dev/null 2>&1; then
  python_cmd=("$PYTHON")
elif command -v python3 >/dev/null 2>&1 && python3 --version >/dev/null 2>&1; then
  python_cmd=(python3)
elif command -v python >/dev/null 2>&1 && python --version >/dev/null 2>&1; then
  python_cmd=(python)
elif command -v py >/dev/null 2>&1 && py -3 --version >/dev/null 2>&1; then
  python_cmd=(py -3)
else
  echo "a working python3 or python is required to summarize trace JSONL" >&2
  exit 1
fi

trace_json_counters="$("${python_cmd[@]}" - "$trace_path" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
reasoning_genome_events = 0
reasoning_genome_write_allowed = 0
reasoning_genome_splice_write_allowed = 0
for index, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
    line = line.strip()
    if not line:
        continue
    try:
        obj = json.loads(line)
    except json.JSONDecodeError as error:
        raise SystemExit(f"{path}:{index} invalid json: {error}")
    genome = obj.get("reasoning_genome")
    if isinstance(genome, dict):
        reasoning_genome_events += 1
        reasoning_genome_write_allowed += int(bool(genome.get("write_allowed", False)))
        reasoning_genome_splice_write_allowed += int(bool(genome.get("splice_write_allowed", False)))

print(
    "reasoning_genome_events={} reasoning_genome_write_allowed={} reasoning_genome_splice_write_allowed={}".format(
        reasoning_genome_events,
        reasoning_genome_write_allowed,
        reasoning_genome_splice_write_allowed,
    )
)
PY
)"
if [[ -z "$trace_json_counters" ]]; then
  echo "trace JSONL counter summary is empty" >&2
  exit 1
fi

trace_passed="$(field_value "$trace_summary_line" passed)"
reasoning_genome_events="$(field_value "$trace_json_counters" reasoning_genome_events)"
reasoning_genome_write_allowed="$(field_value "$trace_json_counters" reasoning_genome_write_allowed)"
reasoning_genome_splice_write_allowed="$(field_value "$trace_json_counters" reasoning_genome_splice_write_allowed)"
self_evolution_admission_events="$(field_value "$trace_summary_line" self_evolution_admission_events)"
self_evolution_admission_review_packets="$(field_value "$trace_summary_line" self_evolution_admission_review_packets)"
self_evolution_admission_evidence_ids="$(field_value "$trace_summary_line" self_evolution_admission_evidence_ids)"
self_evolution_admission_missing_review_packet_refs="$(field_value "$trace_summary_line" self_evolution_admission_missing_review_packet_refs)"
memory_admission_events="$(field_value "$trace_summary_line" memory_admission_events)"
memory_admission_candidates="$(field_value "$trace_summary_line" memory_admission_candidates)"
memory_admission_ledger_records="$(field_value "$trace_summary_line" memory_admission_ledger_records)"
memory_admission_ledger_authorized="$(field_value "$trace_summary_line" memory_admission_ledger_authorized)"
memory_admission_ledger_applied="$(field_value "$trace_summary_line" memory_admission_ledger_applied)"
memory_admission_ledger_preview_only="$(field_value "$trace_summary_line" memory_admission_ledger_preview_only)"
memory_admission_admitted="$(field_value "$trace_summary_line" memory_admission_admitted)"
memory_admission_hold="$(field_value "$trace_summary_line" memory_admission_hold)"
memory_admission_reject="$(field_value "$trace_summary_line" memory_admission_reject)"
memory_admission_ledger_held="$(field_value "$trace_summary_line" memory_admission_ledger_held)"
memory_admission_ledger_rejected="$(field_value "$trace_summary_line" memory_admission_ledger_rejected)"
memory_admission_ledger_duplicate="$(field_value "$trace_summary_line" memory_admission_ledger_duplicate)"
memory_admission_ledger_decayed="$(field_value "$trace_summary_line" memory_admission_ledger_decayed)"
memory_admission_ledger_merged="$(field_value "$trace_summary_line" memory_admission_ledger_merged)"
memory_admission_ledger_rollback="$(field_value "$trace_summary_line" memory_admission_ledger_rollback)"
memory_admission_source_semantic="$(field_value "$trace_summary_line" memory_admission_source_semantic)"
memory_admission_source_gist="$(field_value "$trace_summary_line" memory_admission_source_gist)"
memory_admission_source_runtime_kv="$(field_value "$trace_summary_line" memory_admission_source_runtime_kv)"
memory_admission_source_cold="$(field_value "$trace_summary_line" memory_admission_source_cold)"
memory_admission_source_gene_segment="$(field_value "$trace_summary_line" memory_admission_source_gene_segment)"
memory_admission_gene_segment_metadata="$(field_value "$trace_summary_line" memory_admission_gene_segment_metadata)"
memory_admission_read_only="$(field_value "$trace_summary_line" memory_admission_read_only)"
memory_admission_write_allowed="$(field_value "$trace_summary_line" memory_admission_write_allowed)"
memory_admission_applied="$(field_value "$trace_summary_line" memory_admission_applied)"
memory_retention_activity_cases="$(field_value "$benchmark_summary_line" memory_retention_activity_cases)"
memory_retention_decayed="$(field_value "$benchmark_summary_line" memory_retention_decayed)"
memory_retention_removed="$(field_value "$benchmark_summary_line" memory_retention_removed)"
memory_compaction_activity_cases="$(field_value "$benchmark_summary_line" memory_compaction_activity_cases)"
memory_compaction_merged="$(field_value "$benchmark_summary_line" memory_compaction_merged)"
memory_compaction_removed="$(field_value "$benchmark_summary_line" memory_compaction_removed)"
memory_compaction_pair_evidence="$(field_value "$benchmark_summary_line" memory_compaction_pair_evidence)"
memory_storage_samples="$(field_value "$benchmark_summary_line" memory_storage_samples)"
memory_storage_entries_before="$(field_value "$benchmark_summary_line" memory_storage_entries_before)"
memory_storage_entries_after="$(field_value "$benchmark_summary_line" memory_storage_entries_after)"
memory_storage_entries_removed="$(field_value "$benchmark_summary_line" memory_storage_entries_removed)"
memory_storage_reduction_entries="$(field_value "$benchmark_summary_line" memory_storage_reduction_entries)"
memory_retained_usefulness_abs_delta_milli="$(field_value "$benchmark_summary_line" memory_retained_usefulness_abs_delta_milli)"
require_nonempty trace_schema_gate passed "$trace_passed"
require_nonempty trace_jsonl reasoning_genome_events "$reasoning_genome_events"
require_nonempty trace_jsonl reasoning_genome_write_allowed "$reasoning_genome_write_allowed"
require_nonempty trace_jsonl reasoning_genome_splice_write_allowed "$reasoning_genome_splice_write_allowed"
require_nonempty trace_schema_gate self_evolution_admission_events "$self_evolution_admission_events"
require_nonempty trace_schema_gate self_evolution_admission_review_packets "$self_evolution_admission_review_packets"
require_nonempty trace_schema_gate self_evolution_admission_evidence_ids "$self_evolution_admission_evidence_ids"
require_nonempty trace_schema_gate self_evolution_admission_missing_review_packet_refs "$self_evolution_admission_missing_review_packet_refs"
require_nonempty trace_schema_gate memory_admission_events "$memory_admission_events"
require_nonempty trace_schema_gate memory_admission_candidates "$memory_admission_candidates"
require_nonempty trace_schema_gate memory_admission_ledger_records "$memory_admission_ledger_records"
require_nonempty trace_schema_gate memory_admission_ledger_authorized "$memory_admission_ledger_authorized"
require_nonempty trace_schema_gate memory_admission_ledger_applied "$memory_admission_ledger_applied"
require_nonempty trace_schema_gate memory_admission_ledger_preview_only "$memory_admission_ledger_preview_only"
require_nonempty trace_schema_gate memory_admission_admitted "$memory_admission_admitted"
require_nonempty trace_schema_gate memory_admission_hold "$memory_admission_hold"
require_nonempty trace_schema_gate memory_admission_reject "$memory_admission_reject"
require_nonempty trace_schema_gate memory_admission_ledger_held "$memory_admission_ledger_held"
require_nonempty trace_schema_gate memory_admission_ledger_rejected "$memory_admission_ledger_rejected"
require_nonempty trace_schema_gate memory_admission_ledger_duplicate "$memory_admission_ledger_duplicate"
require_nonempty trace_schema_gate memory_admission_ledger_decayed "$memory_admission_ledger_decayed"
require_nonempty trace_schema_gate memory_admission_ledger_merged "$memory_admission_ledger_merged"
require_nonempty trace_schema_gate memory_admission_ledger_rollback "$memory_admission_ledger_rollback"
require_nonempty trace_schema_gate memory_admission_source_semantic "$memory_admission_source_semantic"
require_nonempty trace_schema_gate memory_admission_source_gist "$memory_admission_source_gist"
require_nonempty trace_schema_gate memory_admission_source_runtime_kv "$memory_admission_source_runtime_kv"
require_nonempty trace_schema_gate memory_admission_source_cold "$memory_admission_source_cold"
require_nonempty trace_schema_gate memory_admission_source_gene_segment "$memory_admission_source_gene_segment"
require_nonempty trace_schema_gate memory_admission_gene_segment_metadata "$memory_admission_gene_segment_metadata"
require_nonempty trace_schema_gate memory_admission_read_only "$memory_admission_read_only"
require_nonempty trace_schema_gate memory_admission_write_allowed "$memory_admission_write_allowed"
require_nonempty trace_schema_gate memory_admission_applied "$memory_admission_applied"
require_nonempty benchmark_summary memory_retention_activity_cases "$memory_retention_activity_cases"
require_nonempty benchmark_summary memory_retention_decayed "$memory_retention_decayed"
require_nonempty benchmark_summary memory_retention_removed "$memory_retention_removed"
require_nonempty benchmark_summary memory_compaction_activity_cases "$memory_compaction_activity_cases"
require_nonempty benchmark_summary memory_compaction_merged "$memory_compaction_merged"
require_nonempty benchmark_summary memory_compaction_removed "$memory_compaction_removed"
require_nonempty benchmark_summary memory_compaction_pair_evidence "$memory_compaction_pair_evidence"
require_nonempty benchmark_summary memory_storage_samples "$memory_storage_samples"
require_nonempty benchmark_summary memory_storage_entries_before "$memory_storage_entries_before"
require_nonempty benchmark_summary memory_storage_entries_after "$memory_storage_entries_after"
require_nonempty benchmark_summary memory_storage_entries_removed "$memory_storage_entries_removed"
require_nonempty benchmark_summary memory_storage_reduction_entries "$memory_storage_reduction_entries"
require_nonempty benchmark_summary memory_retained_usefulness_abs_delta_milli "$memory_retained_usefulness_abs_delta_milli"

cat >"$trace_report" <<EOF
trace_schema_gate: passed=$trace_passed reasoning_genome_events=$reasoning_genome_events reasoning_genome_write_allowed=$reasoning_genome_write_allowed reasoning_genome_splice_write_allowed=$reasoning_genome_splice_write_allowed self_evolution_admission_events=$self_evolution_admission_events self_evolution_admission_review_packets=$self_evolution_admission_review_packets self_evolution_admission_evidence_ids=$self_evolution_admission_evidence_ids self_evolution_admission_missing_review_packet_refs=$self_evolution_admission_missing_review_packet_refs memory_admission_events=$memory_admission_events memory_admission_candidates=$memory_admission_candidates memory_admission_ledger_records=$memory_admission_ledger_records memory_admission_ledger_authorized=$memory_admission_ledger_authorized memory_admission_ledger_applied=$memory_admission_ledger_applied memory_admission_ledger_preview_only=$memory_admission_ledger_preview_only memory_admission_admitted=$memory_admission_admitted memory_admission_hold=$memory_admission_hold memory_admission_reject=$memory_admission_reject memory_admission_ledger_held=$memory_admission_ledger_held memory_admission_ledger_rejected=$memory_admission_ledger_rejected memory_admission_ledger_duplicate=$memory_admission_ledger_duplicate memory_admission_ledger_decayed=$memory_admission_ledger_decayed memory_admission_ledger_merged=$memory_admission_ledger_merged memory_admission_ledger_rollback=$memory_admission_ledger_rollback memory_admission_source_semantic=$memory_admission_source_semantic memory_admission_source_gist=$memory_admission_source_gist memory_admission_source_runtime_kv=$memory_admission_source_runtime_kv memory_admission_source_cold=$memory_admission_source_cold memory_admission_source_gene_segment=$memory_admission_source_gene_segment memory_admission_gene_segment_metadata=$memory_admission_gene_segment_metadata memory_admission_read_only=$memory_admission_read_only memory_admission_write_allowed=$memory_admission_write_allowed memory_admission_applied=$memory_admission_applied disk_kv_compact_reopen_verified=$disk_kv_compact_reopen_verified disk_kv_compact_reopen_test=$disk_kv_compact_reopen_test memory_admission_ledger_reopen_verified=$memory_admission_ledger_reopen_verified memory_admission_ledger_reopen_test=$memory_admission_ledger_reopen_test memory_admission_authorized_fixture_apply_verified=$memory_admission_authorized_fixture_apply_verified memory_admission_authorized_fixture_apply_test=$memory_admission_authorized_fixture_apply_test memory_admission_authorized_fixture_authorized=$memory_admission_authorized_fixture_authorized memory_admission_authorized_fixture_applied=$memory_admission_authorized_fixture_applied memory_admission_authorized_fixture_admitted=$memory_admission_authorized_fixture_admitted memory_admission_authorized_fixture_rehydrated=$memory_admission_authorized_fixture_rehydrated memory_admission_authorized_fixture_reopened_records=$memory_admission_authorized_fixture_reopened_records memory_admission_authorized_fixture_ledger_bytes_nonzero=$memory_admission_authorized_fixture_ledger_bytes_nonzero memory_admission_runtime_preview_apply_verified=$memory_admission_runtime_preview_apply_verified memory_admission_runtime_preview_apply_test=$memory_admission_runtime_preview_apply_test memory_admission_runtime_preview_authorized=$memory_admission_runtime_preview_authorized memory_admission_runtime_preview_applied=$memory_admission_runtime_preview_applied memory_admission_runtime_preview_admitted=$memory_admission_runtime_preview_admitted memory_admission_runtime_preview_rehydrated=$memory_admission_runtime_preview_rehydrated memory_admission_read_only_authorized_append_denied=$memory_admission_read_only_authorized_append_denied memory_admission_read_only_authorized_append_test=$memory_admission_read_only_authorized_append_test memory_admission_read_only_authorized_append_preserved_existing_bytes=$memory_admission_read_only_authorized_append_preserved_existing_bytes memory_admission_review_scope_required_verified=$memory_admission_review_scope_required_verified memory_admission_review_scope_required_test=$memory_admission_review_scope_required_test memory_admission_review_scope_required_tenant_rejection=$memory_admission_review_scope_required_tenant_rejection memory_admission_review_scope_required_session_rejection=$memory_admission_review_scope_required_session_rejection memory_admission_review_scope_required_authorized=$memory_admission_review_scope_required_authorized memory_admission_review_scope_required_appended=$memory_admission_review_scope_required_appended memory_admission_invalid_shape_rejection_verified=$memory_admission_invalid_shape_rejection_verified memory_admission_invalid_shape_rejection_test=$memory_admission_invalid_shape_rejection_test memory_admission_invalid_shape_source_hash_present=$memory_admission_invalid_shape_source_hash_present memory_admission_invalid_shape_kv_shape_valid=$memory_admission_invalid_shape_kv_shape_valid memory_admission_invalid_shape_ledger_rejected=$memory_admission_invalid_shape_ledger_rejected memory_admission_invalid_shape_ledger_authorized=$memory_admission_invalid_shape_ledger_authorized memory_admission_invalid_shape_preview_read_only=$memory_admission_invalid_shape_preview_read_only memory_admission_invalid_shape_preview_write_allowed=$memory_admission_invalid_shape_preview_write_allowed memory_retention_activity_cases=$memory_retention_activity_cases memory_retention_decayed=$memory_retention_decayed memory_retention_removed=$memory_retention_removed memory_compaction_activity_cases=$memory_compaction_activity_cases memory_compaction_merged=$memory_compaction_merged memory_compaction_removed=$memory_compaction_removed memory_compaction_pair_evidence=$memory_compaction_pair_evidence memory_storage_samples=$memory_storage_samples memory_storage_entries_before=$memory_storage_entries_before memory_storage_entries_after=$memory_storage_entries_after memory_storage_entries_removed=$memory_storage_entries_removed memory_storage_reduction_entries=$memory_storage_reduction_entries memory_retained_usefulness_abs_delta_milli=$memory_retained_usefulness_abs_delta_milli memory_autophagy_context_pressure_score=$memory_autophagy_context_pressure_score memory_autophagy_retrieval_noise_score=$memory_autophagy_retrieval_noise_score memory_autophagy_stale_decay_candidates=$memory_autophagy_stale_decay_candidates memory_autophagy_duplicate_merge_candidates=$memory_autophagy_duplicate_merge_candidates memory_autophagy_gist_recomposition_candidates=$memory_autophagy_gist_recomposition_candidates memory_autophagy_active_recall_prune_candidates=$memory_autophagy_active_recall_prune_candidates memory_autophagy_quarantine_candidates=$memory_autophagy_quarantine_candidates memory_autophagy_live_delete_allowed=$memory_autophagy_live_delete_allowed memory_autophagy_durable_mutation_allowed=$memory_autophagy_durable_mutation_allowed memory_autophagy_reason_codes=$memory_autophagy_reason_codes
EOF
trace_report_line="$(cat "$trace_report")"
cat >"$trace_report" <<EOF
$trace_report_line issue503_chaperone_fold_guard_verified=$issue503_chaperone_fold_guard_verified issue503_chaperone_fold_guard_test=$issue503_chaperone_fold_guard_test issue503_fold_status=$issue503_fold_status issue503_undefined_capability_count=$issue503_undefined_capability_count issue503_contradiction_count=$issue503_contradiction_count issue503_ungated_side_effect_count=$issue503_ungated_side_effect_count issue503_missing_evidence_count=$issue503_missing_evidence_count issue503_repair_task_count=$issue503_repair_task_count issue503_raw_cot_captured=$issue503_raw_cot_captured issue503_raw_prompt_captured=$issue503_raw_prompt_captured
EOF

release_review="$smoke_root/release-review.txt"
cat >"$release_review" <<'EOF'
pr=433 review=MERGED checks=passed branch_protection=present
pr=487 review=MERGED checks=passed branch_protection=present
EOF

issue_state="$smoke_root/issue-state.txt"
cat >"$issue_state" <<'EOF'
issue=31 state=open final_signoff=true
issue=19 state=closed runtime_surface_closed=true runtime_surface_merged_prs=#290,#291,#292,#293,#296,#307,#308,#309,#433 runtime_counters_pr=#429 runtime_counters_head=a3668d89eeb200996ec1213d52fe69a5347cd9fe runtime_counters_checks=green runtime_counters_review=merged runtime_counters_merged=true runtime_surface_blocker=none
issue=30 state=closed close_allowed=true
EOF

demo_proof="$smoke_root/demo-proof.txt"
cat >"$demo_proof" <<'EOF'
clean_checkout=true live_model_required=false private_state_required=false prompt_digest_ref=redaction-digest:issue30-default-prompt integration_test=issue30_clean_checkout_demo_writes_digest_only_evidence_packet dispatch_test=issue30_dispatch_roundtrip_inspect_runs_trace_schema_gate dispatch_path=dispatch::run trace_schema_gate_executed=true
EOF

issue30_context="$smoke_root/issue30-context.txt"
cat >"$issue30_context" <<'EOF'
issue30_environment_pressure_present=true issue30_pollution_event_id=redaction-digest:dddddddddddddddd issue385_self_ontology_body_present=true issue385_body_state_id=redaction-digest:eeeeeeeeeeeeeeee issue385_pheromone_signal_marker_present=true issue385_pheromone_signal_marker_id=redaction-digest:9999999999999999 issue385_pheromone_signal_surface=digest_marker issue385_pheromone_signal_digest_gate_allowed=true issue385_pheromone_signal_preview_only=true issue375_pre_reasoning_genome_isa_present=true issue375_reasoning_frame_id=redaction-digest:ffffffffffffffff issue375_reasoning_frame_environment_signals_present=true issue375_reasoning_frame_allowed_observations=repo_issue_terminal_runtime_state issue375_reasoning_frame_action_vocab=observe_inspect_compare_summarize_verify_quarantine issue375_reasoning_frame_suppressed_capabilities=write_process_browser_network_memory_genome_runtime issue375_reasoning_frame_risk_limits=preview_only_digest_only issue375_expression_vm_side_effect=read_only issue375_genome_isa_apply_allowed=false issue30_backend_action=deterministic_runtime_kv_roundtrip issue243_active_control_knobs=routing|context_anchor|suppression|checkpoint|memory_maintenance issue243_evidence_digest=redaction-digest:control243 issue243_policy_version=control_expression_gate_v1 issue243_decision_reason=no_weight_runtime_control_preview issue243_control_expression_profile_selected=1 issue243_context_anchor_promoted=1 issue243_suppression_gate_triggered=1 issue243_checkpoint_repair_requested=1 issue243_checkpoint_rejected=1 issue243_memory_refresh_candidate=1 issue243_memory_tombstone_candidate=1 issue243_control_expression_preview_admission=1 issue243_write_allowed=false issue243_applied=false issue243_operator_approval_required=true issue379_control_candidate_preview_only=true issue379_action_vocab_mask_preview=true issue379_signal_saliency_bias_preview=true issue379_zero_beat_primitive_decision_present=true issue379_primitive_authority=preview_only issue379_primitive_side_effect=read_only issue379_primitive_reversibility=rollback_required issue379_primitive_evidence=digest_only issue379_primitive_uncertainty=hold_on_gap issue379_primitive_attention=focus_or_mask_preview issue379_zero_beat_output=action_vocab_mask_and_signal_saliency_bias issue379_generation_bias_apply_allowed=false issue493_tool_organ_registry_present=true issue493_tool_organ_registry_id=redaction-digest:1111111111111111 issue493_tool_organ_registry_preview_only=true issue493_tool_organ_registry_side_effect=read_only issue493_tool_organ_registry_apply_allowed=false issue493_tool_organ_capability_matrix_digest=redaction-digest:2222222222222222 issue493_preview_bundle_protocol=bundle_v1 issue493_preview_bundle_digest=redaction-digest:3333333333333333 issue493_preview_bundle_refs_digest_only=true issue493_preview_bundle_raw_artifacts_allowed=false issue493_tool_install_allowed=false issue493_tool_execution_allowed=false bio_epigenetic_expression_marker_present=true bio_epigenetic_expression_marker_id=redaction-digest:4444444444444444 bio_mrna_cache_candidate_digest=redaction-digest:5555555555555555 bio_expression_cache_protocol=mrna_preview_v1 bio_expression_cache_key_digest=redaction-digest:6666666666666666 bio_hot_path_observation_window=100 bio_hot_path_min_success_rate=0.98 bio_gate_relaxation_allowed=false bio_cache_materialization_allowed=false bio_raw_payload_or_kv_cached=false bio_negative_evidence_overrides=true issue501_telomere_state_present=true issue501_remaining_tokens=0 issue501_remaining_steps=0 issue501_remaining_messages=0 issue501_repair_streak_count=2 issue501_loop_risk_signal_count=4 issue501_senescent=true issue501_apoptosis_required=true issue501_new_external_call_allowed=false issue501_new_file_write_allowed=false issue501_new_memory_write_allowed=false issue501_new_adaptive_state_write_allowed=false issue501_memory_promotion_allowed=false issue501_genome_mutation_allowed=false issue501_takeover_packet_digest=redaction-digest:7777777777777777 issue501_rollback_anchor_digest=redaction-digest:8888888888888888 issue501_handoff_next_owner=scheduler issue501_raw_payload_present=false issue501_preview_side_effect_allowed=false issue502_pheromone_blackboard_present=true issue502_signal_count=3 issue502_ranked_action_count=3 issue502_top_signal_kind=repair_first issue502_top_action=repair_review issue502_blackboard_digest=redaction-digest:9999999999999999 issue502_source_digest=redaction-digest:aaaaaaaaaaaaaaaa issue502_payload_digest=redaction-digest:bbbbbbbbbbbbbbbb issue502_raw_payload_present=false issue502_side_effect_allowed=false issue502_ttl_decay_present=true issue502_conflict_routes_to_repair=true issue502_ranked_actions_from_state_only=true issue509_quorum_sensing_present=true issue509_decision_id=redaction-digest:9999999999999509 issue509_quorum_report_digest=redaction-digest:aaaaaaaaaaaa0509 issue509_risk_class=irreversible issue509_required_quorum_milli=700 issue509_evaluator_count=3 issue509_independent_model_count=3 issue509_independent_lane_count=3 issue509_approve_signal_count=2 issue509_reject_signal_count=1 issue509_abstain_signal_count=0 issue509_approval_concentration_milli=666 issue509_conflict_count=1 issue509_quorum_reached=false issue509_apply_allowed=false issue509_raw_evaluator_payload_present=false issue509_duplicate_sources_count_once=true issue509_conflict_routes_to_repair=true issue509_writer_gate_bypass_allowed=false
issue377_problem_finding_present=true issue377_problem_finding_id=redaction-digest:aaaaaaaaaaaaaaaa issue377_hypothesis_candidate_present=true issue377_hypothesis_candidate_id=redaction-digest:bbbbbbbbbbbbbbbb issue377_problem_hypothesis_link=redaction-digest:cccccccccccccccc issue377_admission_decision=preview_only issue377_predicament_signal_present=true issue377_predicament_id=redaction-digest:dddddddddddddddd issue377_predicament_progress_delta=0 issue377_predicament_repeat_count=2 issue377_predicament_evidence_gap_count=0 issue377_predicament_action_novelty=0 issue377_predicament_stuck=true issue377_self_trigger_stage=preview_only issue377_evolution_apply_allowed=false
EOF

state_files="$smoke_root/state-files.txt"
ndkv_non_fixture_writes="$(
  find "$repo_root" -type f -name '*.ndkv' \
    ! -path "$repo_root/.git/*" \
    ! -path "$repo_root/target/*" \
    | wc -l \
    | tr -d '[:space:]'
)"
require_nonempty state_files ndkv_non_fixture_writes "$ndkv_non_fixture_writes"
cat >"$state_files" <<EOF
memory=$memory_path experience=$experience_path adaptive=$adaptive_path ndkv_non_fixture_writes=$ndkv_non_fixture_writes
EOF

raw_input="$smoke_root/raw-input.txt"
cat >"$raw_input" <<'EOF'
issue30_fresh_checkout_smoke=passed
EOF

packet="$smoke_root/evidence-packet.md"
display_command='cargo run --locked --package rust-norion -- --benchmark-roundtrip --inspect-state --inspect-gate --trace "$STATE_DIR/issue30-trace.jsonl" --trace-schema-gate "$STATE_DIR/issue30-trace.jsonl" --memory "$STATE_DIR/memory.ndkv" --experience "$STATE_DIR/experience.ndkv" --adaptive "$STATE_DIR/adaptive.ndkv" --profile coding --runtime-kv-exchange --runtime-layers 6 --runtime-hidden-size 64 --runtime-attention-heads 4 --runtime-kv-heads 2 --runtime-local-window 32 --inspect-min-runtime-kv-memories 1 --inspect-min-experiences 1 --inspect-min-runtime-model-experiences 1 --inspect-min-runtime-adapter-experiences 1 --inspect-max-runtime-adapter-selection-mismatches 0 --inspect-min-runtime-forward-energy-experiences 1 --inspect-min-runtime-kv-influence-experiences 1 --inspect-min-runtime-kv-precision-experiences 1 --inspect-max-runtime-kv-precision-mismatches 0 --inspect-min-runtime-device-execution-experiences 1 --inspect-min-runtime-kv-import-experiences 1 --inspect-min-runtime-kv-export-experiences 1 --inspect-min-live-memory-feedback-experiences 1 --inspect-min-live-memory-feedback-updates 1 --inspect-require-runtime-kv-dimensions'

"${cargo_cmd[@]}" run --locked --package norion-cli -- evidence-packet \
  --issue 30 \
  --commit "$(git rev-parse HEAD)" \
  --command "$display_command" \
  --gate passed \
  --input "$raw_input" \
  --git-worktree "$repo_root" \
  --release-review-input "$release_review" \
  --issue-state-input "$issue_state" \
  --demo-proof-input "$demo_proof" \
  --roundtrip-proof-input "$roundtrip_proof" \
  --trace-report-input "$trace_report" \
  --state-gate-input "$state_gate" \
  --issue30-context-input "$issue30_context" \
  --state-files-input "$state_files" \
  --output "$packet" \
  --require 'issue30_fresh_checkout_smoke=passed' \
  --require 'cargo run --locked --package rust-norion -- --benchmark-roundtrip' \
  --require 'dirty_worktree=false' \
  --require 'rc_snapshot_ready=true' \
  --require 'rc_prs=#433,#487' \
  --require 'release_review_ready=true' \
  --require 'release_review_blockers=none' \
  --require 'issue19_runtime_counters_ready=true' \
  --require 'issue19_runtime_counters_state=head_a3668d8_checks_green_merged_merged' \
  --require 'issue19_runtime_surface_closed=true' \
  --require 'issue19_runtime_surface_blocker=none' \
  --require 'issue30_close_allowed=true' \
  --require 'issue30_clean_checkout_demo_ready=true' \
  --require 'issue30_positive_context_loop_ready=true' \
  --require 'issue243_control_expression_gate_ready=true' \
  --require 'issue243_active_control_knobs=routing|context_anchor|suppression|checkpoint|memory_maintenance' \
  --require 'issue243_write_allowed=false' \
  --require 'issue243_applied=false' \
  --require 'issue243_operator_approval_required=true' \
  --require 'issue385_pheromone_signal_marker_present=true' \
  --require 'issue385_pheromone_signal_marker_id=redaction-digest:' \
  --require 'issue385_pheromone_signal_surface=digest_marker' \
  --require 'issue385_pheromone_signal_digest_gate_allowed=true' \
  --require 'issue385_pheromone_signal_preview_only=true' \
  --require 'issue375_reasoning_frame_environment_signals_present=true' \
  --require 'issue375_reasoning_frame_allowed_observations=repo_issue_terminal_runtime_state' \
  --require 'issue375_reasoning_frame_action_vocab=observe_inspect_compare_summarize_verify_quarantine' \
  --require 'issue375_reasoning_frame_suppressed_capabilities=write_process_browser_network_memory_genome_runtime' \
  --require 'issue375_reasoning_frame_risk_limits=preview_only_digest_only' \
  --require 'issue375_expression_vm_side_effect=read_only' \
  --require 'issue375_genome_isa_apply_allowed=false' \
  --require 'issue379_zero_beat_primitive_decision_present=true' \
  --require 'issue379_primitive_authority=preview_only' \
  --require 'issue379_primitive_side_effect=read_only' \
  --require 'issue379_primitive_reversibility=rollback_required' \
  --require 'issue379_primitive_evidence=digest_only' \
  --require 'issue379_primitive_uncertainty=hold_on_gap' \
  --require 'issue379_primitive_attention=focus_or_mask_preview' \
  --require 'issue379_zero_beat_output=action_vocab_mask_and_signal_saliency_bias' \
  --require 'issue379_generation_bias_apply_allowed=false' \
  --require 'issue493_tool_organ_registry_present=true' \
  --require 'issue493_tool_organ_registry_id=redaction-digest:' \
  --require 'issue493_tool_organ_registry_preview_only=true' \
  --require 'issue493_tool_organ_registry_side_effect=read_only' \
  --require 'issue493_tool_organ_registry_apply_allowed=false' \
  --require 'issue493_tool_organ_capability_matrix_digest=redaction-digest:' \
  --require 'issue493_preview_bundle_protocol=bundle_v1' \
  --require 'issue493_preview_bundle_digest=redaction-digest:' \
  --require 'issue493_preview_bundle_refs_digest_only=true' \
  --require 'issue493_preview_bundle_raw_artifacts_allowed=false' \
  --require 'issue493_tool_install_allowed=false' \
  --require 'issue493_tool_execution_allowed=false' \
  --require 'bio_epigenetic_expression_marker_present=true' \
  --require 'bio_epigenetic_expression_marker_id=redaction-digest:' \
  --require 'bio_mrna_cache_candidate_digest=redaction-digest:' \
  --require 'bio_expression_cache_protocol=mrna_preview_v1' \
  --require 'bio_expression_cache_key_digest=redaction-digest:' \
  --require 'bio_hot_path_observation_window=100' \
  --require 'bio_hot_path_min_success_rate=0.98' \
  --require 'bio_gate_relaxation_allowed=false' \
  --require 'bio_cache_materialization_allowed=false' \
  --require 'bio_raw_payload_or_kv_cached=false' \
  --require 'bio_negative_evidence_overrides=true' \
  --require 'issue501_telomere_state_present=true' \
  --require 'issue501_remaining_tokens=0' \
  --require 'issue501_remaining_steps=0' \
  --require 'issue501_remaining_messages=0' \
  --require 'issue501_repair_streak_count=2' \
  --require 'issue501_loop_risk_signal_count=4' \
  --require 'issue501_senescent=true' \
  --require 'issue501_apoptosis_required=true' \
  --require 'issue501_new_external_call_allowed=false' \
  --require 'issue501_new_file_write_allowed=false' \
  --require 'issue501_new_memory_write_allowed=false' \
  --require 'issue501_new_adaptive_state_write_allowed=false' \
  --require 'issue501_memory_promotion_allowed=false' \
  --require 'issue501_genome_mutation_allowed=false' \
  --require 'issue501_takeover_packet_digest=redaction-digest:' \
  --require 'issue501_rollback_anchor_digest=redaction-digest:' \
  --require 'issue501_handoff_next_owner=scheduler' \
  --require 'issue501_raw_payload_present=false' \
  --require 'issue501_preview_side_effect_allowed=false' \
  --require 'issue502_pheromone_blackboard_present=true' \
  --require 'issue502_signal_count=3' \
  --require 'issue502_ranked_action_count=3' \
  --require 'issue502_top_signal_kind=repair_first' \
  --require 'issue502_top_action=repair_review' \
  --require 'issue502_blackboard_digest=redaction-digest:' \
  --require 'issue502_source_digest=redaction-digest:' \
  --require 'issue502_payload_digest=redaction-digest:' \
  --require 'issue502_raw_payload_present=false' \
  --require 'issue502_side_effect_allowed=false' \
  --require 'issue502_ttl_decay_present=true' \
  --require 'issue502_conflict_routes_to_repair=true' \
  --require 'issue502_ranked_actions_from_state_only=true' \
  --require 'issue509_quorum_sensing_present=true' \
  --require 'issue509_decision_id=redaction-digest:' \
  --require 'issue509_quorum_report_digest=redaction-digest:' \
  --require 'issue509_risk_class=irreversible' \
  --require 'issue509_required_quorum_milli=700' \
  --require 'issue509_evaluator_count=3' \
  --require 'issue509_independent_model_count=3' \
  --require 'issue509_independent_lane_count=3' \
  --require 'issue509_approve_signal_count=2' \
  --require 'issue509_reject_signal_count=1' \
  --require 'issue509_abstain_signal_count=0' \
  --require 'issue509_approval_concentration_milli=666' \
  --require 'issue509_conflict_count=1' \
  --require 'issue509_quorum_reached=false' \
  --require 'issue509_apply_allowed=false' \
  --require 'issue509_raw_evaluator_payload_present=false' \
  --require 'issue509_duplicate_sources_count_once=true' \
  --require 'issue509_conflict_routes_to_repair=true' \
  --require 'issue509_writer_gate_bypass_allowed=false' \
  --require 'issue377_predicament_signal_present=true' \
  --require 'issue377_predicament_id=redaction-digest:' \
  --require 'issue377_predicament_progress_delta=0' \
  --require 'issue377_predicament_repeat_count=2' \
  --require 'issue377_predicament_evidence_gap_count=0' \
  --require 'issue377_predicament_action_novelty=0' \
  --require 'issue377_predicament_stuck=true' \
  --require 'issue377_self_trigger_stage=preview_only' \
  --require 'issue377_evolution_apply_allowed=false' \
  --require 'persistent_roundtrip: passed=true' \
  --require 'issue30_second_task_benefit_ready=true' \
  --require 'issue30_negative_gates_ready=true' \
  --require 'trace_schema_gate: passed=true' \
  --require 'memory_admission_events=' \
  --require 'memory_admission_candidates=' \
  --require 'memory_admission_ledger_records=' \
  --require 'memory_admission_ledger_authorized=0' \
  --require 'memory_admission_ledger_applied=0' \
  --require 'memory_admission_write_allowed=0' \
  --require 'memory_admission_applied=0' \
  --require 'issue2_memory_admission_preview_apply_proof=true' \
  --require 'issue2_memory_admission_preview_apply_proof_source=trace_report_input_derived' \
  --require 'issue2_memory_ledger_apply_proof=true' \
  --require 'issue2_memory_ledger_apply_proof_source=trace_report_input_derived' \
  --require 'issue2_memory_ledger_lifecycle_retention_proof=true' \
  --require 'issue2_memory_ledger_lifecycle_retention_proof_source=trace_report_input_derived' \
  --require 'issue2_memory_residency_retention_compaction_proof=true' \
  --require 'issue2_memory_residency_retention_compaction_proof_source=trace_report_input_derived' \
  --require 'memory_retention_activity_cases=' \
  --require 'memory_compaction_activity_cases=' \
  --require 'memory_storage_reduction_entries=' \
  --require 'memory_retained_usefulness_abs_delta_milli=' \
  --require 'memory_autophagy_context_pressure_score=115' \
  --require 'memory_autophagy_retrieval_noise_score=10' \
  --require 'memory_autophagy_stale_decay_candidates=1' \
  --require 'memory_autophagy_duplicate_merge_candidates=1' \
  --require 'memory_autophagy_gist_recomposition_candidates=2' \
  --require 'memory_autophagy_active_recall_prune_candidates=5' \
  --require 'memory_autophagy_quarantine_candidates=3' \
  --require 'memory_autophagy_live_delete_allowed=false' \
  --require 'memory_autophagy_durable_mutation_allowed=false' \
  --require 'memory_autophagy_reason_codes=active_recall_prune_preview|gist_recomposition_preview|quarantine_preview|recycle_preview' \
  --require 'issue499_memory_autophagy_preview_proof=true' \
  --require 'issue499_memory_autophagy_preview_proof_source=trace_report_input_derived' \
  --require 'memory_admission_ledger_preview_only=' \
  --require 'memory_admission_admitted=' \
  --require 'memory_admission_hold=' \
  --require 'memory_admission_reject=' \
  --require 'memory_admission_ledger_held=' \
  --require 'memory_admission_ledger_rejected=' \
  --require 'memory_admission_ledger_duplicate=' \
  --require 'memory_admission_ledger_decayed=' \
  --require 'memory_admission_ledger_merged=' \
  --require 'memory_admission_ledger_rollback=' \
  --require 'memory_admission_source_semantic=' \
  --require 'memory_admission_source_gist=' \
  --require 'memory_admission_source_runtime_kv=' \
  --require 'memory_admission_source_cold=' \
  --require 'memory_admission_source_gene_segment=' \
  --require 'memory_admission_gene_segment_metadata=' \
  --require 'memory_admission_source_total=' \
  --require 'issue2_memory_admission_source_mix_proof=true' \
  --require 'issue2_memory_gene_segment_metadata_proof=true' \
  --require 'issue2_memory_admission_source_mix_proof_source=trace_report_input_derived' \
  --require 'disk_kv_compact_reopen_verified=true' \
  --require 'disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values' \
  --require 'memory_admission_ledger_reopen_verified=true' \
  --require 'memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen' \
  --require 'memory_admission_authorized_fixture_apply_verified=true' \
  --require 'memory_admission_authorized_fixture_apply_test=memory_admission::tests::writer_gate_rehydrates_applied_authorized_records_from_existing_ledger' \
  --require 'memory_admission_authorized_fixture_authorized=1' \
  --require 'memory_admission_authorized_fixture_applied=1' \
  --require 'memory_admission_authorized_fixture_admitted=1' \
  --require 'memory_admission_authorized_fixture_rehydrated=1' \
  --require 'memory_admission_authorized_fixture_reopened_records=1' \
  --require 'memory_admission_authorized_fixture_ledger_bytes_nonzero=true' \
  --require 'issue2_memory_authorized_fixture_apply_proof=true' \
  --require 'issue2_memory_authorized_fixture_apply_proof_source=trace_report_input_derived' \
  --require 'memory_admission_runtime_preview_apply_verified=true' \
  --require 'memory_admission_runtime_preview_apply_test=tests::benchmark_state::runtime_memory_admission_preview_applies_after_approved_writer_policy' \
  --require 'memory_admission_runtime_preview_authorized=10' \
  --require 'memory_admission_runtime_preview_applied=10' \
  --require 'memory_admission_runtime_preview_admitted=10' \
  --require 'memory_admission_runtime_preview_rehydrated=10' \
  --require 'issue2_memory_runtime_preview_apply_proof=true' \
  --require 'issue2_memory_runtime_preview_apply_proof_source=trace_report_input_derived' \
  --require 'memory_admission_read_only_authorized_append_denied=true' \
  --require 'memory_admission_read_only_authorized_append_test=memory_admission::tests::writer_gate_refuses_authorized_append_on_read_only_store' \
  --require 'memory_admission_read_only_authorized_append_preserved_existing_bytes=true' \
  --require 'issue2_memory_read_only_authorized_append_denial_proof=true' \
  --require 'issue2_memory_read_only_authorized_append_denial_proof_source=trace_report_input_derived' \
  --require 'memory_admission_invalid_shape_rejection_verified=true' \
  --require 'memory_admission_invalid_shape_rejection_test=memory_admission::tests::gene_segment_kv_records_reject_invalid_shape_without_write' \
  --require 'memory_admission_invalid_shape_source_hash_present=false' \
  --require 'memory_admission_invalid_shape_kv_shape_valid=false' \
  --require 'memory_admission_invalid_shape_ledger_rejected=1' \
  --require 'memory_admission_invalid_shape_ledger_authorized=0' \
  --require 'memory_admission_invalid_shape_preview_read_only=true' \
  --require 'memory_admission_invalid_shape_preview_write_allowed=false' \
  --require 'issue2_memory_invalid_shape_rejection_proof=true' \
  --require 'issue2_memory_invalid_shape_rejection_proof_source=trace_report_input_derived' \
  --require 'memory_admission_review_scope_required_verified=true' \
  --require 'memory_admission_review_scope_required_test=memory_admission::tests::gene_segment_kv_writer_gate_rejects_missing_review_scope_digests' \
  --require 'memory_admission_review_scope_required_tenant_rejection=review_packet_tenant_scope_digest_missing' \
  --require 'memory_admission_review_scope_required_session_rejection=review_packet_session_scope_digest_missing' \
  --require 'memory_admission_review_scope_required_authorized=0' \
  --require 'memory_admission_review_scope_required_appended=0' \
  --require 'issue2_memory_review_scope_required_proof=true' \
  --require 'issue2_memory_review_scope_required_proof_source=trace_report_input_derived' \
  --require 'issue503_chaperone_fold_guard_ready=true' \
  --require 'issue503_fold_status=repair' \
  --require 'issue503_raw_cot_captured=false' \
  --require 'issue30_memory_ledger_trace_ready=true' \
  --require 'issue30_trace_validation_ready=true' \
  --require 'state_inspection_gate: passed=true' \
  --require 'issue30_state_inspection_ready=true' \
  --require 'memory_file_ndkv=true' \
  --require 'experience_file_ndkv=true' \
  --require 'adaptive_file_ndkv=true' \
  --require 'issue2_state_files_ndkv_proof=true' \
  --require 'issue2_state_files_ndkv_proof_source=state_files_input_derived' \
  --require 'issue2_ndkv_non_fixture_writes=0' \
  --require 'issue2_ndkv_non_fixture_write_proof=true' \
  --require 'issue2_ndkv_non_fixture_write_proof_source=state_files_input' \
  --reject "$smoke_root" \
  --reject 'hidden_cot' \
  --reject 'chain-of-thought' \
  --reject 'raw_prompt' \
  --reject 'reuse_response'

grep -E 'issue30_fresh_checkout_smoke=passed|release_review_ready=true|issue30_second_task_benefit_ready=true|issue30_negative_gates_ready=true|disk_kv_compact_reopen_verified=true|memory_admission_ledger_reopen_verified=true|memory_admission_authorized_fixture_apply_verified=true|memory_admission_authorized_fixture_admitted=1|memory_admission_authorized_fixture_ledger_bytes_nonzero=true|memory_admission_runtime_preview_apply_verified=true|memory_admission_runtime_preview_authorized=10|memory_admission_runtime_preview_applied=10|memory_admission_runtime_preview_admitted=10|memory_admission_runtime_preview_rehydrated=10|memory_admission_read_only_authorized_append_denied=true|memory_admission_invalid_shape_rejection_verified=true|memory_admission_invalid_shape_kv_shape_valid=false|issue2_memory_invalid_shape_rejection_proof=true|memory_admission_review_scope_required_verified=true|memory_admission_candidates=|memory_admission_source_semantic=|memory_admission_source_gist=|memory_admission_source_runtime_kv=|memory_admission_source_cold=|memory_admission_source_gene_segment=|memory_admission_gene_segment_metadata=|memory_admission_source_total=|memory_autophagy_context_pressure_score=115|memory_autophagy_gist_recomposition_candidates=2|memory_autophagy_live_delete_allowed=false|memory_autophagy_durable_mutation_allowed=false|issue499_memory_autophagy_preview_proof=true|issue2_memory_admission_preview_apply_proof=true|issue2_memory_authorized_fixture_apply_proof=true|issue2_memory_runtime_preview_apply_proof=true|issue2_memory_admission_source_mix_proof=true|issue2_memory_gene_segment_metadata_proof=true|issue2_memory_read_only_authorized_append_denial_proof=true|issue2_memory_review_scope_required_proof=true|issue503_chaperone_fold_guard_ready=true|issue503_fold_status=repair|issue503_raw_cot_captured=false|issue2_memory_ledger_apply_proof=true|issue2_memory_ledger_lifecycle_retention_proof=true|issue2_memory_residency_retention_compaction_proof=true|bio_epigenetic_expression_marker_present=true|bio_expression_cache_protocol=mrna_preview_v1|bio_gate_relaxation_allowed=false|bio_cache_materialization_allowed=false|bio_raw_payload_or_kv_cached=false|issue501_telomere_state_present=true|issue501_loop_risk_signal_count=4|issue501_apoptosis_required=true|issue501_new_external_call_allowed=false|issue501_new_adaptive_state_write_allowed=false|issue501_memory_promotion_allowed=false|issue501_genome_mutation_allowed=false|issue30_memory_ledger_trace_ready=true|issue30_trace_validation_ready=true|issue30_state_inspection_ready=true|issue2_state_files_ndkv_proof=true|issue2_ndkv_non_fixture_write_proof=true' "$packet"
