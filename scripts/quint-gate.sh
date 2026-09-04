#!/usr/bin/env bash
#
# scripts/quint-gate.sh — ADR 0003 決定 4 の毎 PR ゲート (docs/adr/0003-quint-operations.md)
# quint CLI 0.32.0 前提。
#
# 実行するチェック (1 つでも失敗したら exit 1。全ステップを最後まで実行してから判定する):
#   1. 全モデルの typecheck (engine_loop / stop_hook / journal_protocol)
#   2. 不変条件 run (シード固定 + --max-samples 明示。ADR 0003 決定 4 のとおり、
#      シード固定でも --max-samples を明示しなければ 1 トレースに縮退するため必須)
#   3. 到達性 witness (負形式: ADR 0003 決定 7 の規約どおり `--invariant "not(<w>)"` を実行し、
#      violation が見つかる (quint の exit code != 0) ことを「witness の経路が実在する = pass」
#      と読み替える反転判定を行う。[ok] で終わる (exit code == 0) ときは経路が見つからな
#      かったことを意味するので fail とする)
#   4. 決定的シナリオ (`quint test formal/orchestration/stop_hook.qnt --match 'r_.*'`)
#
# 各ステップの pass/fail を [PASS]/[FAIL] で標準出力に印字する。
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

ENGINE_LOOP="formal/orchestration/engine_loop.qnt"
STOP_HOOK="formal/orchestration/stop_hook.qnt"
JOURNAL_PROTOCOL="formal/orchestration/journal_protocol.qnt"

OVERALL=0
STEP_NAMES=()
STEP_RESULTS=()

log_step() {
  printf '==> %s\n' "$1"
}

# ステップ結果を記録し、その場で [PASS]/[FAIL] を印字する。
record() {
  local name="$1"
  local result="$2"
  STEP_NAMES+=("${name}")
  STEP_RESULTS+=("${result}")
  if [[ "${result}" != "PASS" ]]; then
    OVERALL=1
  fi
  printf '[%s] %s\n' "${result}" "${name}"
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'エラー: 必須コマンド %s が見つかりません\n' "$1" >&2
    exit 1
  }
}
require_cmd quint

# --- 1. typecheck ------------------------------------------------------------
for model in "${ENGINE_LOOP}" "${STOP_HOOK}" "${JOURNAL_PROTOCOL}"; do
  log_step "typecheck: ${model}"
  if quint typecheck "${model}"; then
    record "typecheck ${model}" "PASS"
  else
    record "typecheck ${model}" "FAIL"
  fi
done

# --- 2. 不変条件 run (シード固定 + --max-samples 明示) --------------------------
log_step "invariants run: engine_loop"
if quint run "${ENGINE_LOOP}" --seed 0x1a2b3c --max-samples 2000 --max-steps 40 \
  --invariants no_run_stage_for_skip cursor_in_scope no_gate_bypass \
    gate_lifecycle_preconditions parked_position parked_marker_status \
    unpark_restores_position stale_rereport_yields_done stale_rereport_frame \
    at_most_one_active; then
  record "invariants run: engine_loop" "PASS"
else
  record "invariants run: engine_loop" "FAIL"
fi

log_step "invariants run: stop_hook"
if quint run "${STOP_HOOK}" --seed 0xabc --max-samples 3000 --max-steps 60 \
  --invariants blocked_below_cap cap_is_mode_dependent record_before_decision \
    reset_on_terminal carve_beats_forwarding signature_resets_count \
    parked_autonomous_guard seed2_on_active_chain; then
  record "invariants run: stop_hook" "PASS"
else
  record "invariants run: stop_hook" "FAIL"
fi

log_step "invariants run: journal_protocol"
if quint run "${JOURNAL_PROTOCOL}" --seed 0x5e1 --max-samples 3000 --max-steps 50 \
  --invariants conflict_rejected snapshot_tracks_journal version_equals_journal \
    checkpoint_monotone checkpoint_bounded projection_idempotent truth_is_journal \
    no_lost_update; then
  record "invariants run: journal_protocol" "PASS"
else
  record "invariants run: journal_protocol" "FAIL"
fi

# --- 3. 到達性 witness (負形式: violation = 経路実在 = pass という反転判定) -----------
run_witness() {
  local model="$1" seed="$2" max_samples="$3" max_steps="$4" witness="$5"
  log_step "witness (negated): ${model} :: ${witness}"
  # 終了コードではなく**出力の判定語**で分岐する。quint は解析・型検査エラーや未知の名前
  # (witness 名の typo) でも非 0 で終わるため、終了コードだけを見ると「エラー = 反例あり =
  # PASS」と誤判定する (PR #101 の CodeRabbit 指摘)。
  local output
  output="$(quint run "${model}" --seed "${seed}" --max-samples "${max_samples}" \
    --max-steps "${max_steps}" --invariant "not(${witness})" 2>&1)" || true
  if grep -q '\[violation\]' <<<"${output}"; then
    # [violation] -> not(witness) の反例 = witness が成立する経路が実在 -> pass
    record "witness ${witness} (${model})" "PASS"
  elif grep -q '\[ok\]' <<<"${output}"; then
    # [ok] -> not(witness) が常に成立 -> witness の経路が見つからない -> fail
    record "witness ${witness} (${model})" "FAIL"
  else
    # どちらでもない = quint 自体が走れていない (構文・型・未知の名前)。理由を見せて fail。
    printf '%s\n' "${output}" | tail -n 5 >&2
    record "witness ${witness} (${model}) — quint error" "FAIL"
  fi
}

for w in w_repark; do
  run_witness "${ENGINE_LOOP}" "0x303" 2000 40 "${w}"
done

for w in w_block w_cap_release_interactive w_parked_auto_block w_seed2 w_sig_reset; do
  run_witness "${STOP_HOOK}" "0x9" 3000 60 "${w}"
done

for w in w_conflict w_crash_then_catchup w_interleaved_writers w_idempotent_catchup; do
  run_witness "${JOURNAL_PROTOCOL}" "0x5e1" 5000 50 "${w}"
done

# --- 4. 決定的シナリオ ---------------------------------------------------------
log_step "deterministic scenarios: quint test --match 'r_.*'"
if quint test "${STOP_HOOK}" --match 'r_.*'; then
  record "quint test --match 'r_.*' (${STOP_HOOK})" "PASS"
else
  record "quint test --match 'r_.*' (${STOP_HOOK})" "FAIL"
fi

# --- summary -------------------------------------------------------------------
printf '\n==> summary\n'
i=0
while [[ "${i}" -lt "${#STEP_NAMES[@]}" ]]; do
  printf '  [%s] %s\n' "${STEP_RESULTS[$i]}" "${STEP_NAMES[$i]}"
  i=$((i + 1))
done

if [[ "${OVERALL}" -eq 0 ]]; then
  printf '\n[PASS] quint gate: all steps green\n'
else
  printf '\n[FAIL] quint gate: one or more steps failed\n' >&2
fi

exit "${OVERALL}"
