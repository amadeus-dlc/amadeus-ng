#!/usr/bin/env bash
#
# scripts/coverage.sh — workspace line coverage ゲート (cargo-llvm-cov ベース)
#
# 参考実装: https://raw.githubusercontent.com/j5ik2o/fraktor-rs/main/scripts/coverage.sh
# (構成 — SCRIPT_DIR/REPO_ROOT 解決・log_step・cargo-llvm-cov ラップ — を踏襲しつつ、
#  本リポジトリはゲート判定 (絶対/相対) が主目的のため、レポート出力形式の選択肢は持たない)
#
# ゲート方針 (Issue #6):
#   - 絶対ゲート: workspace 全体の line coverage が ABSOLUTE_THRESHOLD (%) 以上であること。
#     しきい値は下の変数で一元管理する。90% 未達なら exit 1。
#   - 相対ゲート (--base <git-ref> を指定した場合のみ有効): base ref を `git worktree add` で
#     一時ディレクトリにチェックアウトし、head と同条件で計測して比較する。
#     head が base を下回ったら fail (同値・上回りは pass)。浮動小数比較の許容誤差は
#     TOLERANCE。計測後は worktree を必ず削除する (trap で保証)。
#   - 引数なし = 絶対ゲートのみ。`--base <ref>` を付けると絶対 + 相対の両方を実行する。
#
# 計測対象・除外方針:
#   - `cargo llvm-cov --workspace` の既定に従い、workspace 全クレートのプロダクトコードを
#     対象とする。テストコード自体は cargo-llvm-cov の既定で計測対象に含まれない。
#   - `formal/` (Quint モデル) と `docs/` は Rust クレートではないため、workspace の
#     コンパイル対象に含まれず、自然に計測対象外となる。
#   - 明示的な除外は composition root (`modules/app/aidlc/src/main.rs`) の 1 ファイルだけ
#     (NFR2.5)。配線コードはテストで駆動する対象ではないため床の計算から外す。それ以外は
#     90% 床を維持する。除外はファイル単位で、クレート単位 (`--exclude`) にはしない。
#
# 計測の決定化 (NFR2.4):
#   - PBT (proptest) のランダム経路が実行毎に変わると同一コードでも line coverage が
#     揺れる。`PROPTEST_RNG_SEED` を固定して head / base を同条件で計測する。CI 側
#     (.github/workflows/ci.yml) も同じ値を渡すのでローカルと CI の値が一致する。
#
# bash 3.2 (macOS 標準) 互換のため、連想配列・mapfile・readarray は使用しない。
#
set -euo pipefail

# --- しきい値 (ここで一元管理) ----------------------------------------------
ABSOLUTE_THRESHOLD=90.0
# 相対ゲートの許容誤差。PBT のシードを固定して計測を決定化したので (下の
# PROPTEST_RNG_SEED)、同一コードの再計測は差 0.00pp になる。従来の 0.5 は
# シード非固定時の揺れ (2026-08-22 実測: 94.87〜95.29%、±0.4pp) に合わせた
# 較正値であり、決定化後は浮動小数の丸め分だけを見込んだ 0.01 で足りる (NFR2.4)。
TOLERANCE=0.01
# PBT (proptest) の RNG シード。proptest 1.11 はこの環境変数を RngSeed::Fixed(u64)
# として読む。値を変えると計測されるランダム経路が変わるため、変更は PR でのみ行う。
PROPTEST_RNG_SEED="20260823"
export PROPTEST_RNG_SEED
# カバレッジ計測から外す唯一のファイル (composition root、NFR2.5)。
#
# llvm-cov はカバレッジデータに**絶対パス**を記録するため (実測: 2026-08-23、
# `/Users/.../docs/modules/app/aidlc/src/main.rs`)、リポジトリルート相対を意図した
# `^modules/...` 単独のアンカーではどのパスにも一致せず除外が効かない。相対パス基準の
# 意図 (= リポジトリルート直下の modules/... というパス断片に限定し、別クレートの
# 同名ファイルを巻き込まない) を保ったまま実効化するため、行頭またはパス区切りを
# 先頭アンカーに使う。相対ゲートの base 側は一時 worktree の別の絶対パスで計測される
# ので、この形でないと head と base で除外条件がずれる。
IGNORE_FILENAME_REGEX='(^|/)modules/app/aidlc/src/main\.rs$'
# -----------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

BASE_REF=""
WORKTREE_DIR=""

usage() {
  cat <<EOF
使い方: scripts/coverage.sh [オプション]

オプション:
  --base <git-ref>   相対ゲートを有効化する。指定した ref を一時 worktree に
                      チェックアウトし、head と同条件で line coverage を計測して比較する。
  --help, -h          このヘルプを表示

しきい値 (スクリプト冒頭の変数で管理):
  絶対ゲート: ABSOLUTE_THRESHOLD=${ABSOLUTE_THRESHOLD} (%) 以上で pass。
  相対ゲート: head が base を下回ったら fail (同値・上回りは pass)。許容誤差 TOLERANCE=${TOLERANCE}。
  計測除外: ${IGNORE_FILENAME_REGEX} (composition root のみ)。
  PBT シード: PROPTEST_RNG_SEED=${PROPTEST_RNG_SEED} (固定 — 再計測で差 0.00pp)。

例:
  scripts/coverage.sh
  scripts/coverage.sh --base origin/main
EOF
}

log_step() {
  printf '==> %s\n' "$1"
}

fail() {
  printf 'エラー: %s\n' "$1" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "必須コマンド '$1' が見つかりません"
}

cleanup() {
  if [[ -n "${WORKTREE_DIR}" && -d "${WORKTREE_DIR}" ]]; then
    git -C "${REPO_ROOT}" worktree remove --force "${WORKTREE_DIR}" >/dev/null 2>&1 || rm -rf "${WORKTREE_DIR}"
  fi
}
trap cleanup EXIT

# 指定ディレクトリで cargo llvm-cov を実行し、workspace 全体の line coverage %
# (0-100 の小数、例: 95.20264681555004) を標準出力へ印字する。
measure_line_coverage() {
  local dir="$1"
  local tmpdir
  # BSD (macOS 標準) と GNU で `mktemp -t` の意味が異なる (テンプレート解釈が非互換) ため、
  # 両方で同じ結果になる `-t` なしの `mktemp -d` を使う。
  tmpdir="$(mktemp -d)"
  local json_path="${tmpdir}/coverage.json"

  (
    cd "${dir}"
    cargo llvm-cov clean --workspace >/dev/null 2>&1 || true
    cargo llvm-cov --workspace --ignore-filename-regex "${IGNORE_FILENAME_REGEX}" \
      --json --summary-only --output-path "${json_path}" 1>&2
  )

  local percent
  percent="$(jq -r '.data[0].totals.lines.percent' "${json_path}")"
  rm -rf "${tmpdir}"

  if [[ -z "${percent}" || "${percent}" == "null" ]]; then
    fail "line coverage の取得に失敗しました (${dir})"
  fi

  printf '%s' "${percent}"
}

# a >= b - tol なら 0 (真) を返す。bash は浮動小数比較ができないため awk を使う。
ge_with_tolerance() {
  local a="$1" b="$2" tol="$3"
  awk -v a="${a}" -v b="${b}" -v t="${tol}" 'BEGIN { exit !(a >= (b - t)) }'
}

main() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --base)
        [[ $# -ge 2 ]] || fail "--base には git-ref を指定してください"
        BASE_REF="$2"
        shift 2
        ;;
      --help|-h)
        usage
        exit 0
        ;;
      *)
        usage
        fail "未知の引数 '$1'"
        ;;
    esac
  done

  require_cmd cargo
  require_cmd jq
  require_cmd git
  cargo llvm-cov --version >/dev/null 2>&1 || fail "cargo-llvm-cov が見つかりません (cargo install cargo-llvm-cov --locked)"

  local exit_code=0

  log_step "head の line coverage を計測中 (${REPO_ROOT})"
  local head_pct
  head_pct="$(measure_line_coverage "${REPO_ROOT}")"
  printf 'head line coverage: %s%%\n' "${head_pct}"

  if ge_with_tolerance "${head_pct}" "${ABSOLUTE_THRESHOLD}" 0; then
    printf '[PASS] absolute gate: head (%s%%) >= threshold (%s%%)\n' "${head_pct}" "${ABSOLUTE_THRESHOLD}"
  else
    printf '[FAIL] absolute gate: head (%s%%) < threshold (%s%%)\n' "${head_pct}" "${ABSOLUTE_THRESHOLD}"
    exit_code=1
  fi

  if [[ -n "${BASE_REF}" ]]; then
    log_step "base (${BASE_REF}) を一時 worktree にチェックアウト中"
    WORKTREE_DIR="$(mktemp -d)"
    # mktemp で作られる空ディレクトリが残っていると `git worktree add` が失敗するため削除する
    rmdir "${WORKTREE_DIR}"
    git -C "${REPO_ROOT}" worktree add --detach "${WORKTREE_DIR}" "${BASE_REF}" 1>&2

    log_step "base の line coverage を計測中 (${WORKTREE_DIR})"
    local base_pct
    base_pct="$(measure_line_coverage "${WORKTREE_DIR}")"
    printf 'base (%s) line coverage: %s%%\n' "${BASE_REF}" "${base_pct}"

    if ge_with_tolerance "${head_pct}" "${base_pct}" "${TOLERANCE}"; then
      printf '[PASS] relative gate: head (%s%%) >= base (%s%%) - tolerance (%s)\n' "${head_pct}" "${base_pct}" "${TOLERANCE}"
    else
      printf '[FAIL] relative gate: head (%s%%) < base (%s%%) - tolerance (%s)\n' "${head_pct}" "${base_pct}" "${TOLERANCE}"
      exit_code=1
    fi
  fi

  exit "${exit_code}"
}

main "$@"
