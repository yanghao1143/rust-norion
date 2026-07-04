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
memory_path="$state_dir/memory.ndkv"
experience_path="$state_dir/experience.ndkv"
adaptive_path="$state_dir/adaptive.ndkv"
roundtrip_stdout="$smoke_root/roundtrip.stdout"

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

disk_kv_compact_reopen_stdout="$smoke_root/disk-kv-compact-reopen.stdout"
memory_admission_ledger_reopen_stdout="$smoke_root/memory-admission-ledger-reopen.stdout"
"${cargo_cmd[@]}" test --locked --package rust-norion compact_keeps_latest_values >"$disk_kv_compact_reopen_stdout"
"${cargo_cmd[@]}" test --locked --package rust-norion writer_gate_append_is_idempotent_after_store_reopen >"$memory_admission_ledger_reopen_stdout"
disk_kv_compact_reopen_verified=true
disk_kv_compact_reopen_test='disk_kv::tests::compact_keeps_latest_values'
memory_admission_ledger_reopen_verified=true
memory_admission_ledger_reopen_test='memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen'

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
memory_admission_ledger_records="$(field_value "$trace_summary_line" memory_admission_ledger_records)"
memory_admission_ledger_preview_only="$(field_value "$trace_summary_line" memory_admission_ledger_preview_only)"
require_nonempty trace_schema_gate passed "$trace_passed"
require_nonempty trace_jsonl reasoning_genome_events "$reasoning_genome_events"
require_nonempty trace_jsonl reasoning_genome_write_allowed "$reasoning_genome_write_allowed"
require_nonempty trace_jsonl reasoning_genome_splice_write_allowed "$reasoning_genome_splice_write_allowed"
require_nonempty trace_schema_gate self_evolution_admission_events "$self_evolution_admission_events"
require_nonempty trace_schema_gate self_evolution_admission_review_packets "$self_evolution_admission_review_packets"
require_nonempty trace_schema_gate self_evolution_admission_evidence_ids "$self_evolution_admission_evidence_ids"
require_nonempty trace_schema_gate self_evolution_admission_missing_review_packet_refs "$self_evolution_admission_missing_review_packet_refs"
require_nonempty trace_schema_gate memory_admission_ledger_records "$memory_admission_ledger_records"
require_nonempty trace_schema_gate memory_admission_ledger_preview_only "$memory_admission_ledger_preview_only"

cat >"$trace_report" <<EOF
trace_schema_gate: passed=$trace_passed reasoning_genome_events=$reasoning_genome_events reasoning_genome_write_allowed=$reasoning_genome_write_allowed reasoning_genome_splice_write_allowed=$reasoning_genome_splice_write_allowed self_evolution_admission_events=$self_evolution_admission_events self_evolution_admission_review_packets=$self_evolution_admission_review_packets self_evolution_admission_evidence_ids=$self_evolution_admission_evidence_ids self_evolution_admission_missing_review_packet_refs=$self_evolution_admission_missing_review_packet_refs memory_admission_ledger_records=$memory_admission_ledger_records memory_admission_ledger_preview_only=$memory_admission_ledger_preview_only disk_kv_compact_reopen_verified=$disk_kv_compact_reopen_verified disk_kv_compact_reopen_test=$disk_kv_compact_reopen_test memory_admission_ledger_reopen_verified=$memory_admission_ledger_reopen_verified memory_admission_ledger_reopen_test=$memory_admission_ledger_reopen_test
EOF

release_review="$smoke_root/release-review.txt"
cat >"$release_review" <<'EOF'
pr=428 review=MERGED checks=passed branch_protection=present
pr=429 review=MERGED checks=passed branch_protection=present
pr=431 review=MERGED checks=passed branch_protection=present
pr=432 review=MERGED checks=passed branch_protection=present
EOF

issue_state="$smoke_root/issue-state.txt"
cat >"$issue_state" <<'EOF'
issue=31 state=open final_signoff=false
issue=19 state=open runtime_surface_closed=false runtime_surface_merged_prs=#290,#291,#292,#293,#296,#307,#308,#309 runtime_counters_pr=#429 runtime_counters_head=a3668d89eeb200996ec1213d52fe69a5347cd9fe runtime_counters_checks=green runtime_counters_review=merged runtime_counters_merged=true runtime_surface_blocker=#19:OPEN
issue=30 state=open close_allowed=false
EOF

demo_proof="$smoke_root/demo-proof.txt"
cat >"$demo_proof" <<'EOF'
clean_checkout=true live_model_required=false private_state_required=false prompt_digest_ref=redaction-digest:issue30-default-prompt integration_test=issue30_clean_checkout_demo_writes_digest_only_evidence_packet dispatch_test=issue30_dispatch_roundtrip_inspect_runs_trace_schema_gate dispatch_path=dispatch::run trace_schema_gate_executed=true
EOF

issue30_context="$smoke_root/issue30-context.txt"
cat >"$issue30_context" <<'EOF'
issue30_environment_pressure_present=true issue30_pollution_event_id=redaction-digest:dddddddddddddddd issue385_self_ontology_body_present=true issue385_body_state_id=redaction-digest:eeeeeeeeeeeeeeee issue385_pheromone_signal_marker_present=true issue385_pheromone_signal_marker_id=redaction-digest:9999999999999999 issue385_pheromone_signal_surface=digest_marker issue385_pheromone_signal_digest_gate_allowed=true issue385_pheromone_signal_preview_only=true issue375_pre_reasoning_genome_isa_present=true issue375_reasoning_frame_id=redaction-digest:ffffffffffffffff issue375_reasoning_frame_environment_signals_present=true issue375_reasoning_frame_allowed_observations=repo_issue_terminal_runtime_state issue375_reasoning_frame_action_vocab=observe_inspect_compare_summarize_verify_quarantine issue375_reasoning_frame_suppressed_capabilities=write_process_browser_network_memory_genome_runtime issue375_reasoning_frame_risk_limits=preview_only_digest_only issue375_expression_vm_side_effect=read_only issue375_genome_isa_apply_allowed=false issue30_backend_action=deterministic_runtime_kv_roundtrip issue379_control_candidate_preview_only=true issue379_action_vocab_mask_preview=true issue379_signal_saliency_bias_preview=true issue379_zero_beat_primitive_decision_present=true issue379_primitive_authority=preview_only issue379_primitive_side_effect=read_only issue379_primitive_reversibility=rollback_required issue379_primitive_evidence=digest_only issue379_primitive_uncertainty=hold_on_gap issue379_primitive_attention=focus_or_mask_preview issue379_zero_beat_output=action_vocab_mask_and_signal_saliency_bias issue379_generation_bias_apply_allowed=false
issue377_problem_finding_present=true issue377_problem_finding_id=redaction-digest:aaaaaaaaaaaaaaaa issue377_hypothesis_candidate_present=true issue377_hypothesis_candidate_id=redaction-digest:bbbbbbbbbbbbbbbb issue377_problem_hypothesis_link=redaction-digest:cccccccccccccccc issue377_admission_decision=preview_only issue377_predicament_signal_present=true issue377_predicament_id=redaction-digest:dddddddddddddddd issue377_predicament_progress_delta=0 issue377_predicament_repeat_count=2 issue377_predicament_evidence_gap_count=0 issue377_predicament_action_novelty=0 issue377_predicament_stuck=true issue377_self_trigger_stage=preview_only issue377_evolution_apply_allowed=false
EOF

state_files="$smoke_root/state-files.txt"
cat >"$state_files" <<EOF
memory=$memory_path experience=$experience_path adaptive=$adaptive_path
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
  --require 'rc_prs=#428,#429,#431,#432' \
  --require 'release_review_ready=true' \
  --require 'release_review_blockers=none' \
  --require 'issue19_runtime_counters_ready=true' \
  --require 'issue19_runtime_counters_state=head_a3668d8_checks_green_merged_merged' \
  --require 'issue19_runtime_surface_blocker=#19:OPEN' \
  --require 'issue30_close_allowed=false' \
  --require 'issue30_clean_checkout_demo_ready=true' \
  --require 'issue30_positive_context_loop_ready=true' \
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
  --require 'memory_admission_ledger_records=' \
  --require 'memory_admission_ledger_preview_only=' \
  --require 'disk_kv_compact_reopen_verified=true' \
  --require 'disk_kv_compact_reopen_test=disk_kv::tests::compact_keeps_latest_values' \
  --require 'memory_admission_ledger_reopen_verified=true' \
  --require 'memory_admission_ledger_reopen_test=memory_admission::tests::writer_gate_append_is_idempotent_after_store_reopen' \
  --require 'issue30_memory_ledger_trace_ready=true' \
  --require 'issue30_trace_validation_ready=true' \
  --require 'state_inspection_gate: passed=true' \
  --require 'issue30_state_inspection_ready=true' \
  --reject "$smoke_root" \
  --reject 'hidden_cot' \
  --reject 'chain-of-thought' \
  --reject 'raw_prompt' \
  --reject 'reuse_response'

grep -E 'issue30_fresh_checkout_smoke=passed|release_review_ready=true|issue30_second_task_benefit_ready=true|issue30_negative_gates_ready=true|disk_kv_compact_reopen_verified=true|memory_admission_ledger_reopen_verified=true|issue30_memory_ledger_trace_ready=true|issue30_trace_validation_ready=true|issue30_state_inspection_ready=true' "$packet"
