#!/usr/bin/env bash
#
# scripts/governance/verify-ci-governance.sh — CI ガバナンス設定の機械検査 (U10)
#
# packaging Unit の「テスト」に相当する。リポジトリに書かれた**設定の事実**
# (toolchain 固定・workspace lints・CI ワークフロー・カバレッジゲート・GitHub ruleset)
# が U10 の要求どおりかを検査し、失敗項目を列挙して非 0 で終了する。
#
# 検査対象と要求 ID (../../aidlc/spaces/default/intents/260822-stage1-selfhost/
# construction/u10-ci-governance/nfr-requirements/security-requirements.md):
#   rust-toolchain.toml          NFR4.2
#   Cargo.toml / tools/lint      NFR4.3
#   .github/workflows/ci.yml     NFR2.2 / NFR2.3 / NFR2.4 / NFR4.1 / NFR4.2 / NFR4.4
#   scripts/coverage.sh          NFR2.4 / NFR2.5
#   GitHub ruleset (--with-ruleset)  NFR2.1 / NFR4.5
#
# 既定ではネットワークにアクセスしない。--with-ruleset を付けたときだけ gh api を叩く。
#
# bash 3.2 (macOS 標準) 互換のため、連想配列・mapfile・readarray は使用しない。
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# --- 期待値 (ここで一元管理) --------------------------------------------------
EXPECTED_CHANNEL="1.95.0"
EXPECTED_COMPONENTS="rustfmt clippy llvm-tools"
EXPECTED_PROFILE="minimal"
EXPECTED_TOLERANCE="0.05"
# cargo llvm-cov に渡す除外正規表現。llvm-cov は絶対パスを記録するため先頭アンカーは
# 行頭またはパス区切り ((^|/)) にする (scripts/coverage.sh のコメント参照)。
EXPECTED_IGNORE_REGEX='(^|/)modules/app/aidlc/src/main\.rs$'
EXPECTED_SEED="20260823"
EXPECTED_CONTEXTS="check coverage quint" # 比較は集合 (ソート済み) で行う
GOVERNANCE_REPO="${GOVERNANCE_REPO:-amadeus-dlc/amadeus-ng}"
RULESET_NAME="${RULESET_NAME:-main}"
# -----------------------------------------------------------------------------

TOOLCHAIN_FILE="rust-toolchain.toml"
ROOT_MANIFEST="Cargo.toml"
LINT_MANIFEST="tools/lint/Cargo.toml"
CI_FILE=".github/workflows/ci.yml"
COVERAGE_FILE="scripts/coverage.sh"

WITH_RULESET=0

# --- 結果の記録 ---------------------------------------------------------------
# bash 3.2 では連想配列が使えないため、行文字列の配列で保持する。
RESULT_LINES=()
FAIL_COUNT=0
PASS_COUNT=0

pass() {
  RESULT_LINES+=("[PASS] $1 — $2")
  PASS_COUNT=$((PASS_COUNT + 1))
}

fail() {
  RESULT_LINES+=("[FAIL] $1 — $2")
  FAIL_COUNT=$((FAIL_COUNT + 1))
}

# 条件が真なら PASS、偽なら FAIL を記録する (検査 1 項目 = 1 呼出)。
# 使い方: expect <真偽値(0/1)> <id> <説明> <失敗理由>
expect() {
  if [[ "$1" -eq 0 ]]; then
    pass "$2" "$3"
  else
    fail "$2" "$4"
  fi
}

usage() {
  cat <<EOF
使い方: scripts/governance/verify-ci-governance.sh [オプション]

CI ガバナンス設定 (U10) が要求どおりかを機械検査する。

オプション:
  --with-ruleset   GitHub ruleset の required status checks も検査する
                    (gh コマンドとネットワークが必要。既定では検査しない)
  --help, -h       このヘルプを表示

環境変数:
  GOVERNANCE_REPO  ruleset 検査の対象リポジトリ (既定: ${GOVERNANCE_REPO})
  RULESET_NAME     ruleset 名 (既定: ${RULESET_NAME})

終了コード: 全項目 PASS で 0、1 項目でも FAIL があれば 1、引数エラーは 2。
EOF
}

# --- 小道具 -------------------------------------------------------------------

# TOML の指定セクション本文だけを取り出す。 例: toml_section Cargo.toml workspace.lints.rust
toml_section() {
  awk -v sec="[$2]" '
    /^[[:space:]]*\[/ { in_sec = ($0 ~ "^[[:space:]]*\\" sec "[[:space:]]*$"); next }
    in_sec { print }
  ' "$1"
}

# YAML のトップレベルキー配下のブロックだけを取り出す。 例: yaml_top_block ci.yml permissions
yaml_top_block() {
  awk -v k="$2" '
    $0 ~ "^" k ":[[:space:]]*$" { inb = 1; next }
    /^[^[:space:]#]/ { inb = 0 }
    inb { print }
  ' "$1"
}

# ファイルが読めなければ、そのファイル担当の全項目を FAIL で埋めて 1 を返す
# (呼び元は return する)。「ファイルが無い」を黙って PASS にしないための共通処理。
require_file() {
  local path="$1"; shift
  if [[ -f "${path}" ]]; then
    return 0
  fi
  local id
  for id in "$@"; do
    fail "${id}" "${path} が存在しない"
  done
  return 1
}

# --- 検査: rust-toolchain.toml (NFR4.2) ---------------------------------------
check_toolchain_file() {
  require_file "${TOOLCHAIN_FILE}" toolchain-channel toolchain-components toolchain-profile || return 0

  local section
  section="$(toml_section "${TOOLCHAIN_FILE}" "toolchain")"

  if printf '%s\n' "${section}" | grep -Eq "^[[:space:]]*channel[[:space:]]*=[[:space:]]*\"${EXPECTED_CHANNEL}\"[[:space:]]*$"; then
    pass "toolchain-channel" "${TOOLCHAIN_FILE} の channel が \"${EXPECTED_CHANNEL}\" に固定されている"
  else
    fail "toolchain-channel" "${TOOLCHAIN_FILE} の [toolchain] に channel = \"${EXPECTED_CHANNEL}\" が無い"
  fi

  local missing="" comp
  for comp in ${EXPECTED_COMPONENTS}; do
    printf '%s\n' "${section}" | grep -q "\"${comp}\"" || missing="${missing} ${comp}"
  done
  if [[ -z "${missing}" ]]; then
    pass "toolchain-components" "${TOOLCHAIN_FILE} の components に ${EXPECTED_COMPONENTS} が揃っている"
  else
    fail "toolchain-components" "${TOOLCHAIN_FILE} の components に不足:${missing}"
  fi

  if printf '%s\n' "${section}" | grep -Eq "^[[:space:]]*profile[[:space:]]*=[[:space:]]*\"${EXPECTED_PROFILE}\"[[:space:]]*$"; then
    pass "toolchain-profile" "${TOOLCHAIN_FILE} の profile が \"${EXPECTED_PROFILE}\""
  else
    fail "toolchain-profile" "${TOOLCHAIN_FILE} の [toolchain] に profile = \"${EXPECTED_PROFILE}\" が無い"
  fi
}

# --- 検査: workspace lints / tools-lint lints (NFR4.3) ------------------------
check_workspace_lints() {
  if require_file "${ROOT_MANIFEST}" workspace-unsafe-forbid; then
    if toml_section "${ROOT_MANIFEST}" "workspace.lints.rust" \
      | grep -Eq '^[[:space:]]*unsafe_code[[:space:]]*=[[:space:]]*"forbid"[[:space:]]*$'; then
      pass "workspace-unsafe-forbid" "${ROOT_MANIFEST} の [workspace.lints.rust] に unsafe_code = \"forbid\""
    else
      fail "workspace-unsafe-forbid" "${ROOT_MANIFEST} の [workspace.lints.rust] に unsafe_code = \"forbid\" が無い"
    fi
  fi

  if require_file "${LINT_MANIFEST}" tools-lint-unsafe-forbid; then
    if toml_section "${LINT_MANIFEST}" "lints.rust" \
      | grep -Eq '^[[:space:]]*unsafe_code[[:space:]]*=[[:space:]]*"forbid"[[:space:]]*$'; then
      pass "tools-lint-unsafe-forbid" "${LINT_MANIFEST} の [lints.rust] に unsafe_code = \"forbid\""
    else
      fail "tools-lint-unsafe-forbid" "${LINT_MANIFEST} の [lints.rust] に unsafe_code = \"forbid\" が無い (detached クレートは workspace lints を継承しない)"
    fi
  fi
}

# --- 検査: CI ワークフロー ----------------------------------------------------
check_ci_workflow() {
  require_file "${CI_FILE}" \
    ci-merge-group-trigger ci-permissions-contents-read ci-toolchain-file-driven \
    ci-tools-lint-steps ci-audit-job ci-proptest-seed-env ci-coverage-base-condition || return 0

  # NFR2.2: merge queue のチェックを走らせるための merge_group トリガ
  if yaml_top_block "${CI_FILE}" "on" | grep -Eq '^[[:space:]]*merge_group:'; then
    pass "ci-merge-group-trigger" "on: に merge_group トリガがある (NFR2.2)"
  else
    fail "ci-merge-group-trigger" "on: に merge_group トリガが無い (merge queue のチェックが走らない)"
  fi

  # NFR4.4: 最小権限
  if yaml_top_block "${CI_FILE}" "permissions" | grep -Eq '^[[:space:]]*contents:[[:space:]]*read[[:space:]]*$'; then
    pass "ci-permissions-contents-read" "workflow 直下に permissions: contents: read がある (NFR4.4)"
  else
    fail "ci-permissions-contents-read" "workflow 直下に permissions: contents: read が無い (既定権限のまま)"
  fi

  # NFR4.2: toolchain は rust-toolchain.toml 駆動 — @master へ渡す toolchain / components 入力は
  # scripts/governance/toolchain-inputs.sh がファイルから導出した step 出力だけを使い、
  # ci.yml にバージョンやコンポーネントのリテラルを書かない (正本は 1 つ)。
  local tc_ok=0
  grep -q 'dtolnay/rust-toolchain@master' "${CI_FILE}" || tc_ok=1
  grep -q 'dtolnay/rust-toolchain@stable' "${CI_FILE}" && tc_ok=1
  grep -q 'scripts/governance/toolchain-inputs.sh' "${CI_FILE}" || tc_ok=1
  grep -Eq '^[[:space:]]*toolchain:[[:space:]]*\$\{\{ steps\.toolchain\.outputs\.channel \}\}' "${CI_FILE}" || tc_ok=1
  grep -Eq '^[[:space:]]*components:[[:space:]]*\$\{\{ steps\.toolchain\.outputs\.components \}\}' "${CI_FILE}" || tc_ok=1
  grep -Eq '^[[:space:]]*toolchain:[[:space:]]*"?[0-9]' "${CI_FILE}" && tc_ok=1
  grep -Eq '^[[:space:]]*components:[[:space:]]*[a-z]' "${CI_FILE}" && tc_ok=1
  if [[ -f "scripts/governance/toolchain-inputs.sh" ]]; then
    local derived
    derived="$(bash scripts/governance/toolchain-inputs.sh "${TOOLCHAIN_FILE}" 2>/dev/null || true)"
    printf '%s\n' "${derived}" | grep -q "^channel=${EXPECTED_CHANNEL}$" || tc_ok=1
  else
    tc_ok=1
  fi
  expect "${tc_ok}" "ci-toolchain-file-driven" \
    "toolchain が dtolnay/rust-toolchain@master + rust-toolchain.toml 駆動 (toolchain / components はファイルから導出、NFR4.2)" \
    "toolchain が rust-toolchain.toml 駆動になっていない (@master が無い / @stable が残っている / toolchain-inputs.sh 経由でない / リテラルの toolchain: や components: が残っている)"

  # NFR2.3: tools/lint (detached クレート) の fmt / clippy / 自己テスト
  local tl_ok=0
  grep -q 'cargo fmt --manifest-path tools/lint/Cargo.toml --all --check' "${CI_FILE}" || tl_ok=1
  grep -q 'cargo clippy --manifest-path tools/lint/Cargo.toml --all-targets -- -D warnings' "${CI_FILE}" || tl_ok=1
  grep -q 'cargo test --manifest-path tools/lint/Cargo.toml' "${CI_FILE}" || tl_ok=1
  expect "${tl_ok}" "ci-tools-lint-steps" \
    "check ジョブに tools/lint の fmt / clippy / test 3 ステップがある (NFR2.3)" \
    "check ジョブに tools/lint の fmt / clippy / test 3 ステップが揃っていない"

  # NFR4.1: cargo audit を 2 つの Cargo.lock に対して実行する audit ジョブ
  local au_ok=0
  grep -Eq '^[[:space:]]{2}audit:[[:space:]]*$' "${CI_FILE}" || au_ok=1
  grep -Eq '^[[:space:]]*tool:[[:space:]]*cargo-audit[[:space:]]*$' "${CI_FILE}" || au_ok=1
  grep -Eq '^[[:space:]]*run:[[:space:]]*cargo audit[[:space:]]*$' "${CI_FILE}" || au_ok=1
  grep -q 'cargo audit --file tools/lint/Cargo.lock' "${CI_FILE}" || au_ok=1
  expect "${au_ok}" "ci-audit-job" \
    "audit ジョブが cargo-audit を導入し 2 つの Cargo.lock を監査する (NFR4.1)" \
    "audit ジョブが無い / cargo-audit の導入・2 ロックファイルの監査が揃っていない"

  # NFR2.4: PBT シードを CI でも固定する (check / coverage の両方)
  local seed_hits
  seed_hits="$(grep -c "PROPTEST_RNG_SEED: \"${EXPECTED_SEED}\"" "${CI_FILE}" || true)"
  if [[ "${seed_hits}" -ge 2 ]]; then
    pass "ci-proptest-seed-env" "check / coverage に PROPTEST_RNG_SEED: \"${EXPECTED_SEED}\" がある (${seed_hits} 箇所、NFR2.4)"
  else
    fail "ci-proptest-seed-env" "PROPTEST_RNG_SEED: \"${EXPECTED_SEED}\" の指定が ${seed_hits} 箇所 (check / coverage の 2 箇所以上が必要)"
  fi

  # NFR2.2 / NFR2.5: 相対ゲートは pull_request のときだけ (merge_group は base ref を持たない)
  local cov_ok=0
  grep -q "if: github.event_name == 'pull_request'" "${CI_FILE}" || cov_ok=1
  grep -q "if: github.event_name != 'pull_request'" "${CI_FILE}" || cov_ok=1
  grep -q 'bash scripts/coverage.sh --base' "${CI_FILE}" || cov_ok=1
  expect "${cov_ok}" "ci-coverage-base-condition" \
    "coverage は pull_request のときだけ --base を使い、それ以外は絶対ゲートのみ (NFR2.2)" \
    "coverage の pull_request 分岐 (--base の有無) が揃っていない"
}

# --- 検査: カバレッジゲート (NFR2.4 / NFR2.5) ---------------------------------
check_coverage_script() {
  require_file "${COVERAGE_FILE}" \
    coverage-tolerance coverage-ignore-regex coverage-proptest-seed || return 0

  if grep -Eq "^TOLERANCE=${EXPECTED_TOLERANCE}$" "${COVERAGE_FILE}"; then
    pass "coverage-tolerance" "TOLERANCE=${EXPECTED_TOLERANCE} に引き締められている (NFR2.4)"
  else
    fail "coverage-tolerance" "TOLERANCE=${EXPECTED_TOLERANCE} でない (暫定 0.05 — Bolt B2 ゲート裁定。U3 ロック退役後に 0.01)"
  fi

  # 除外は「--ignore-filename-regex を cargo llvm-cov に渡している」ことと
  # 「その正規表現が期待どおり composition root 1 ファイルに限定されている」ことの
  # 2 つの事実で確認する (インライン記述でも定数経由でも成立するようにする)。
  # パターンが -- で始まるためオプション扱いされないよう -e で渡す。
  local ig_ok=0
  grep -q -e '--ignore-filename-regex' "${COVERAGE_FILE}" || ig_ok=1
  grep -F -q -e "${EXPECTED_IGNORE_REGEX}" "${COVERAGE_FILE}" || ig_ok=1
  expect "${ig_ok}" "coverage-ignore-regex" \
    "cargo llvm-cov に --ignore-filename-regex '${EXPECTED_IGNORE_REGEX}' を渡し composition root だけを除外している (NFR2.5)" \
    "--ignore-filename-regex '${EXPECTED_IGNORE_REGEX}' の指定が無い (composition root の除外が未設定)"

  if grep -Eq "^PROPTEST_RNG_SEED=\"?${EXPECTED_SEED}\"?$" "${COVERAGE_FILE}" \
    && grep -Eq '^export PROPTEST_RNG_SEED$|^export PROPTEST_RNG_SEED=' "${COVERAGE_FILE}"; then
    pass "coverage-proptest-seed" "PROPTEST_RNG_SEED=${EXPECTED_SEED} を export して計測を決定化している (NFR2.4)"
  else
    fail "coverage-proptest-seed" "PROPTEST_RNG_SEED=${EXPECTED_SEED} の export が無い (PBT のシードが固定されず計測が揺れる)"
  fi
}

# --- 検査: GitHub ruleset (NFR2.1 / NFR4.5、--with-ruleset のときだけ) --------
check_ruleset() {
  local id="ruleset-required-checks"

  if ! command -v gh >/dev/null 2>&1; then
    fail "${id}" "gh コマンドが見つからない (--with-ruleset には gh が必要)"
    return
  fi
  if ! command -v jq >/dev/null 2>&1; then
    fail "${id}" "jq コマンドが見つからない"
    return
  fi

  local list ruleset_id detail actual strict
  if ! list="$(gh api "repos/${GOVERNANCE_REPO}/rulesets" 2>&1)"; then
    fail "${id}" "ruleset 一覧の取得に失敗 (${GOVERNANCE_REPO}): $(printf '%s' "${list}" | head -1)"
    return
  fi
  ruleset_id="$(printf '%s' "${list}" | jq -r --arg n "${RULESET_NAME}" 'map(select(.name == $n)) | .[0].id // empty')"
  if [[ -z "${ruleset_id}" ]]; then
    fail "${id}" "ruleset「${RULESET_NAME}」が ${GOVERNANCE_REPO} に見つからない"
    return
  fi
  if ! detail="$(gh api "repos/${GOVERNANCE_REPO}/rulesets/${ruleset_id}" 2>&1)"; then
    fail "${id}" "ruleset の取得に失敗 (id=${ruleset_id}): $(printf '%s' "${detail}" | head -1)"
    return
  fi

  actual="$(printf '%s' "${detail}" \
    | jq -r '[.rules[] | select(.type == "required_status_checks") | .parameters.required_status_checks[].context] | sort | join(" ")')"
  strict="$(printf '%s' "${detail}" \
    | jq -r '[.rules[] | select(.type == "required_status_checks") | .parameters.strict_required_status_checks_policy] | .[0] // false')"

  if [[ "${actual}" == "${EXPECTED_CONTEXTS}" && "${strict}" == "true" ]]; then
    pass "${id}" "ruleset「${RULESET_NAME}」(id=${ruleset_id}) の required checks が [${actual}] + strict (NFR2.1)"
  else
    fail "${id}" "ruleset「${RULESET_NAME}」(id=${ruleset_id}) の required checks が期待と違う (実際: [${actual:-なし}] strict=${strict} / 期待: [${EXPECTED_CONTEXTS}] strict=true)"
  fi
}

main() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --with-ruleset) WITH_RULESET=1; shift ;;
      --help|-h) usage; exit 0 ;;
      *) usage >&2; printf 'エラー: 未知の引数 %s\n' "$1" >&2; exit 2 ;;
    esac
  done

  cd "${REPO_ROOT}"

  check_toolchain_file
  check_workspace_lints
  check_ci_workflow
  check_coverage_script
  if [[ "${WITH_RULESET}" -eq 1 ]]; then
    check_ruleset
  fi

  printf '=== CI ガバナンス検査 (%s) ===\n' "${REPO_ROOT}"
  if [[ "${#RESULT_LINES[@]}" -gt 0 ]]; then
    local line
    for line in "${RESULT_LINES[@]}"; do
      printf '%s\n' "${line}"
    done
  fi
  printf -- '--- 合計: PASS %d / FAIL %d ---\n' "${PASS_COUNT}" "${FAIL_COUNT}"

  if [[ "${FAIL_COUNT}" -gt 0 ]]; then
    exit 1
  fi
  exit 0
}

main "$@"
