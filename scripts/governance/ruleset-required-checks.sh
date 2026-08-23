#!/usr/bin/env bash
#
# scripts/governance/ruleset-required-checks.sh — GitHub ruleset に required status
# checks を設定する冪等スクリプト (NFR2.1 / NFR4.5 — U10 CI ガバナンス)
#
# 何をするか:
#   1. ruleset「main」を名前で解決し、変更前の JSON を取得して before.json に保存する。
#   2. 既存の規則 (deletion / non_fast_forward / merge_queue) と bypass_actors を
#      維持したまま、required_status_checks (check / quint / coverage、strict) を
#      追加 / 補正した PUT 用 JSON を組み立てる。
#      ※ PUT /repos/{owner}/{repo}/rulesets/{id} は rules[] を**全置換**するため、
#         既存規則を載せ直さないと消える。
#   3. 冪等判定: 現在の required コンテキスト**集合**と strict フラグが期待どおりなら
#      PUT せずに終了する (規則 type の有無ではなく中身で判定する — nfr-design レビュー
#      Minor 2 の引き取り)。
#   4. PUT 後に取り直した JSON を after.json に保存し、jq で検証して終了コードに反映する。
#
# 既定は安全側: --dry-run を付けると組み立てた JSON を印字するだけで PUT しない。
# 実行にはリポジトリ管理者権限の gh 認証が要る。
#
# bash 3.2 (macOS 標準) 互換のため、連想配列・mapfile・readarray は使用しない。
#
set -euo pipefail

# --- 期待値 (ここで一元管理) --------------------------------------------------
REPO="${GOVERNANCE_REPO:-amadeus-dlc/amadeus-ng}"
RULESET_NAME="${RULESET_NAME:-main}"
# required にするチェックのコンテキスト名 (= ci.yml のジョブ名)。
# audit は含めない — advisory DB / ネットワークの一時障害で全マージが止まらないよう
# required 外に置く裁定 (nfr-design §2)。
# カンマ区切り (コンテキスト名に空白を含むため)。「CI Success」は review-thread ゲートを含む集約ジョブ
# (オーナー指示 2026-08-22 UTC — レビュースレッド未解決の PR をマージさせない)。
REQUIRED_CONTEXTS="check,quint,coverage,CI Success"
STRICT_POLICY="true"
# -----------------------------------------------------------------------------

DRY_RUN=0
OUT_DIR=""

usage() {
  cat <<EOF
使い方: scripts/governance/ruleset-required-checks.sh [オプション]

GitHub ruleset「${RULESET_NAME}」に required status checks (${REQUIRED_CONTEXTS}) を
冪等に設定する。既存の規則と bypass_actors は維持する。

オプション:
  --dry-run          PUT を実行せず、組み立てた JSON を印字するだけにする
  --out-dir <dir>    変更前後の ruleset JSON を <dir>/before.json, <dir>/after.json に保存する。
                     --dry-run のときは after.json の代わりに組み立てた PUT ペイロードを <dir>/planned.json に保存する
  --help, -h         このヘルプを表示

環境変数:
  GOVERNANCE_REPO    対象リポジトリ (既定: ${REPO})
  RULESET_NAME       ruleset 名 (既定: ${RULESET_NAME})

終了コード: 期待どおりの状態に収束して 0、検証失敗や API エラーで 1、引数エラーで 2。
必要な権限: リポジトリ管理者 (ruleset の読み書き)。--dry-run は読取のみ。
EOF
}

log_step() {
  printf '==> %s\n' "$1" >&2
}

die() {
  printf 'エラー: %s\n' "$1" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "必須コマンド '$1' が見つかりません"
}

# REQUIRED_CONTEXTS を JSON 配列 [{"context":"check"}, ...] に変換する。
contexts_json() {
  local ctx json="[]" ctxs
  IFS=',' read -r -a ctxs <<<"${REQUIRED_CONTEXTS}"
  for ctx in "${ctxs[@]}"; do
    json="$(printf '%s' "${json}" | jq --arg c "${ctx}" '. + [{context: $c}]')"
  done
  printf '%s' "${json}"
}

# 期待するコンテキスト集合 (ソート済み・| 区切り — 名前に空白を含むため)。冪等判定の左辺。
expected_context_set() {
  local ctx json="[]" ctxs
  IFS=',' read -r -a ctxs <<<"${REQUIRED_CONTEXTS}"
  for ctx in "${ctxs[@]}"; do
    json="$(printf '%s' "${json}" | jq --arg c "${ctx}" '. + [$c]')"
  done
  printf '%s' "${json}" | jq -r 'sort | join("|")'
}

# ruleset JSON から現在のコンテキスト集合 (ソート済み・| 区切り) を取り出す。
actual_context_set() {
  printf '%s' "$1" | jq -r '
    [.rules[] | select(.type == "required_status_checks")
              | .parameters.required_status_checks[]?.context] | sort | join("|")'
}

# ruleset JSON から strict フラグを取り出す (規則が無ければ false)。
actual_strict_policy() {
  printf '%s' "$1" | jq -r '
    [.rules[] | select(.type == "required_status_checks")
              | .parameters.strict_required_status_checks_policy] | .[0] // false'
}

# 変更前 JSON から PUT 用のペイロードを組み立てる。
# GET のレスポンスには id / node_id / _links など PUT に送れないフィールドが含まれるため、
# PUT が受け付けるフィールドだけを選び直す。rules[] は全置換なので既存規則を載せ直す。
build_payload() {
  local before="$1" rule
  rule="$(jq -n \
    --argjson contexts "$(contexts_json)" \
    --argjson strict "${STRICT_POLICY}" \
    '{type: "required_status_checks",
      parameters: {strict_required_status_checks_policy: $strict,
                   required_status_checks: $contexts}}')"
  printf '%s' "${before}" | jq --argjson rule "${rule}" '
    {name, target, enforcement, conditions, bypass_actors,
     rules: ((.rules | map(select(.type != "required_status_checks"))) + [$rule])}'
}

save_json() {
  local content="$1" name="$2"
  [[ -n "${OUT_DIR}" ]] || return 0
  mkdir -p "${OUT_DIR}"
  printf '%s\n' "${content}" | jq '.' > "${OUT_DIR}/${name}"
  log_step "${OUT_DIR}/${name} に保存しました"
}

main() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --dry-run) DRY_RUN=1; shift ;;
      --out-dir)
        [[ $# -ge 2 ]] || die "--out-dir にはディレクトリを指定してください"
        OUT_DIR="$2"; shift 2 ;;
      --help|-h) usage; exit 0 ;;
      *) usage >&2; printf 'エラー: 未知の引数 %s\n' "$1" >&2; exit 2 ;;
    esac
  done

  require_cmd gh
  require_cmd jq

  log_step "ruleset「${RULESET_NAME}」を ${REPO} から解決中"
  local list ruleset_id before
  list="$(gh api "repos/${REPO}/rulesets" 2>&1)" \
    || die "ruleset 一覧の取得に失敗しました: $(printf '%s' "${list}" | head -3)"
  ruleset_id="$(printf '%s' "${list}" | jq -r --arg n "${RULESET_NAME}" \
    'map(select(.name == $n)) | .[0].id // empty')"
  [[ -n "${ruleset_id}" ]] || die "ruleset「${RULESET_NAME}」が ${REPO} に見つかりません"

  before="$(gh api "repos/${REPO}/rulesets/${ruleset_id}" 2>&1)" \
    || die "ruleset の取得に失敗しました (id=${ruleset_id}): $(printf '%s' "${before}" | head -3)"
  save_json "${before}" "before.json"

  local expected actual strict
  expected="$(expected_context_set)"
  actual="$(actual_context_set "${before}")"
  strict="$(actual_strict_policy "${before}")"
  printf '現在の required checks: [%s] strict=%s\n' "${actual:-なし}" "${strict}" >&2
  printf '期待する required checks: [%s] strict=%s\n' "${expected}" "${STRICT_POLICY}" >&2

  if [[ "${actual}" == "${expected}" && "${strict}" == "${STRICT_POLICY}" ]]; then
    log_step "既に期待どおりの状態です (冪等 — 変更しません)"
    save_json "${before}" "after.json"
    exit 0
  fi

  local payload
  payload="$(build_payload "${before}")"

  if [[ "${DRY_RUN}" -eq 1 ]]; then
    log_step "--dry-run: PUT は実行しません。組み立てた JSON を印字します"
    printf '%s\n' "${payload}"
    if [[ -n "${OUT_DIR}" ]]; then
      save_json "${payload}" "planned.json"
    fi
    exit 0
  fi

  log_step "PUT /repos/${REPO}/rulesets/${ruleset_id} を実行中"
  local put_out
  put_out="$(printf '%s' "${payload}" \
    | gh api --method PUT "repos/${REPO}/rulesets/${ruleset_id}" --input - 2>&1)" \
    || die "ruleset の更新に失敗しました: $(printf '%s' "${put_out}" | head -3)"

  local after
  after="$(gh api "repos/${REPO}/rulesets/${ruleset_id}" 2>&1)" \
    || die "更新後の ruleset の取得に失敗しました: $(printf '%s' "${after}" | head -3)"
  save_json "${after}" "after.json"

  local after_actual after_strict
  after_actual="$(actual_context_set "${after}")"
  after_strict="$(actual_strict_policy "${after}")"

  # 既存規則が全置換で消えていないことも併せて検証する。
  local kept rule_type missing=""
  for rule_type in deletion non_fast_forward merge_queue; do
    kept="$(printf '%s' "${after}" | jq -r --arg t "${rule_type}" \
      '[.rules[] | select(.type == $t)] | length')"
    [[ "${kept}" -ge 1 ]] || missing="${missing} ${rule_type}"
  done

  if [[ "${after_actual}" == "${expected}" && "${after_strict}" == "${STRICT_POLICY}" && -z "${missing}" ]]; then
    printf '[PASS] required checks: [%s] strict=%s、既存規則も維持されています\n' "${after_actual}" "${after_strict}"
    exit 0
  fi

  printf '[FAIL] 期待どおりに収束しませんでした (実際: [%s] strict=%s%s)\n' \
    "${after_actual:-なし}" "${after_strict}" \
    "${missing:+, 消えた既存規則:${missing}}" >&2
  exit 1
}

main "$@"
