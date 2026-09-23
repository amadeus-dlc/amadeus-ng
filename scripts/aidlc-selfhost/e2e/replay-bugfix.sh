#!/usr/bin/env bash
# bugfix 1 周の再生 — stage-1 の受入テスト。
#
# 配布の `.claude/` 手順書（2.8.2）が指揮役に打たせるコマンド列を、指定した aidlc 実体へ
# そのまま流す。Claude Code が発火させるフック（UserPromptSubmit の record-human-turn、
# PostToolUse の write-audit-log / rebuild-stage-graph）は同じ入力 JSON で擬似発火させる。
# 人間の返答は record-human-turn フックを通して入れる（承認ガードを外さない）。
#
# 正解は配布 2.8.2 の実体である。`--engine ~/.local/share/aidlc/versions/2.8.2/aidlc` で
# 全段が期待どおりになることを確かめてから、同じ列を native へ当てる。
#
#   scripts/aidlc-selfhost/e2e/replay-bugfix.sh --engine target/release/aidlc --out /tmp/replay-native
#   scripts/aidlc-selfhost/e2e/compare-directives.sh /tmp/replay-dist /tmp/replay-native
#
# 各段は `ok`（受理されるべき）か `refuse`（拒否されるべき）の期待を持つ。期待と違った段を
# 数え、1 件でもあれば終了コード 1 で終える。記録は <out>/transcript.log、各ステージの
# run-stage 指示は <out>/directives/<stage>.json に残る。
set -u

usage() { echo "usage: $0 --engine <aidlc binary> --out <dir> [--repo <repo root>]" >&2; exit 2; }

ENGINE="" OUT="" REPO=""
while [ $# -gt 0 ]; do
  case "$1" in
    --engine) ENGINE="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    --repo) REPO="$2"; shift 2 ;;
    *) usage ;;
  esac
done
[ -n "$ENGINE" ] && [ -n "$OUT" ] || usage
[ -x "$ENGINE" ] || { echo "engine is not executable: $ENGINE" >&2; exit 2; }
command -v jq >/dev/null || { echo "jq is required" >&2; exit 2; }
[ -n "$REPO" ] || REPO="$(git rev-parse --show-toplevel)"
ENGINE="$(cd "$(dirname "$ENGINE")" && pwd)/$(basename "$ENGINE")"

rm -rf "$OUT"
mkdir -p "$OUT/bin" "$OUT/directives"
OUT="$(cd "$OUT" && pwd)"
ln -s "$ENGINE" "$OUT/bin/aidlc"
# 砂場はコミット済みの木から作る。作業ツリーの未コミット変更（手元の settings.json など）を
# 持ち込まない。ディレクトリ名は両実体で同じ `repo` にする（codekb/<repo> の名前が揃う）。
git clone --quiet --local --no-hardlinks "$REPO" "$OUT/repo"
cd "$OUT/repo" || exit 2
export PATH="$OUT/bin:$PATH"
export CLAUDE_PROJECT_DIR="$PWD"
SESSION="replay-1"
LOG="$OUT/transcript.log"
LAST="$OUT/last.out"
: > "$LOG"
FAILED=0
STEPS=0

log() { printf '%s\n' "$*" >> "$LOG"; }

# 出力が拒否か。report は拒否でも終了コード 0 で kind:error / guard-recovery ask を返す。
is_refusal() {
  local rc="$1"
  [ "$rc" -ne 0 ] && return 0
  jq -e 'type == "object" and (has("error") or .kind == "error" or (.kind == "ask" and .ask_type == "guard-recovery"))' "$LAST" >/dev/null 2>&1
}

# step <ok|refuse|any> <説明> -- <aidlc 引数...>   (any は記録だけして数えない)
# aidlc を実行し、Claude Code が Bash の後に発火させる PostToolUse フックを続けて発火させる。
step() {
  local expect="$1" desc="$2"; shift 3
  local rc verdict
  STEPS=$((STEPS + 1))
  aidlc "$@" > "$LAST" 2> "$LAST.err"; rc=$?
  cat "$LAST.err" >> "$LAST"
  if is_refusal "$rc"; then verdict=refuse; else verdict=ok; fi
  if [ "$expect" = any ]; then
    STEPS=$((STEPS - 1)); log "NOTE [got $verdict] $desc :: aidlc $*"
  elif [ "$verdict" = "$expect" ]; then
    log "PASS [$expect] $desc :: aidlc $*"
  else
    FAILED=$((FAILED + 1))
    log "FAIL [expected $expect, got $verdict] $desc :: aidlc $*"
    echo "FAIL [expected $expect, got $verdict] $desc" >&2
  fi
  jq -c 'del(.rules_content?, .conductor_persona?)' "$LAST" >> "$LOG" 2>/dev/null || head -c 2000 "$LAST" >> "$LOG"
  hook_post_bash "aidlc $*"
  return 0
}

# check <説明> <シェル式> — 砂場の状態に対する期待。
check() {
  local desc="$1" expr="$2"
  STEPS=$((STEPS + 1))
  if eval "$expr"; then log "PASS [check] $desc"; else
    FAILED=$((FAILED + 1)); log "FAIL [check] $desc :: $expr"; echo "FAIL [check] $desc" >&2
  fi
}

hook_post_bash() {
  jq -nc --arg cwd "$PWD" --arg cmd "$1" --rawfile o "$LAST" --arg s "$SESSION" \
    '{session_id:$s,transcript_path:"/dev/null",cwd:$cwd,hook_event_name:"PostToolUse",tool_name:"Bash",tool_input:{command:$cmd},tool_response:{stdout:$o,stderr:"",interrupted:false}}' \
    | aidlc engine hook rebuild-stage-graph >/dev/null 2>&1
}

# human <text> — 人間の発話。UserPromptSubmit の record-human-turn を発火させる。
human() {
  jq -nc --arg p "$1" --arg cwd "$PWD" --arg s "$SESSION" \
    '{session_id:$s,transcript_path:"/dev/null",cwd:$cwd,hook_event_name:"UserPromptSubmit",prompt:$p}' \
    | aidlc engine hook record-human-turn >/dev/null 2>>"$LOG"
  log "HUMAN: $1"
}

# wrote <relpath> — Write ツールの後の PostToolUse（write-audit-log）。
# run-sensors は配布のまま残す 2 本の片方（hook-binding.json）なので、ここでは発火させない。
wrote() {
  local abs="$PWD/$1"
  jq -nc --arg f "$abs" --arg cwd "$PWD" --rawfile c "$abs" --arg s "$SESSION" \
    '{session_id:$s,transcript_path:"/dev/null",cwd:$cwd,hook_event_name:"PostToolUse",tool_name:"Write",tool_input:{file_path:$f,content:$c},tool_response:{filePath:$f,success:true}}' \
    | aidlc engine hook write-audit-log >/dev/null 2>>"$LOG"
}

# write <relpath> — 標準入力を書いて write フックを発火させる。
write() { mkdir -p "$(dirname "$1")"; cat > "$1"; wrote "$1"; }

# advance <tag> — next を叩き、load-steering を continue で送り切って run-stage 等を得る。
advance() {
  step ok "next ($1)" -- engine orchestrate next
  while [ "$(jq -r '.kind // empty' "$LAST" 2>/dev/null)" = "load-steering" ]; do
    step ok "continue ($1)" -- engine orchestrate continue "$(jq -r .continue_token "$LAST")"
  done
  cp "$LAST" "$OUT/directives/$1.json"
  cp "$LAST" "$OUT/current.json"
}

cur() { jq -r "$1" "$OUT/current.json"; }

# ステージ開始時の日記。エンジンが作るのが契約（conductor-persona「Keeping the diary」）。
diary_check() {
  check "$1: エンジンが memory.md を作っている" "[ -f \"$(cur .memory_path)\" ]"
  local m; m="$(cur .memory_path)"
  [ -f "$m" ] || { mkdir -p "$(dirname "$m")"; cp .claude/knowledge/aidlc-shared/memory-template.md "$m"; }
}

# 要約確認（stage-protocol.md の PRE-GENERATION SUMMARY STOP）。
summary_confirm() {
  local st="$1" q="$2"
  write "$q" <<'EOF'
# Questions

## Question 1
どの状況で起きますか？

A. active intent が無いとき
B. 常に
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

active intent が無いとき statusline が空文字になる不具合を直す。

[Answer]:
EOF
  step ok "$st: 要約確認の提示を記録" -- engine log decision --stage "$st" --checkpoint summary-confirmation \
    --questions-file "$q" --decision "Does this all look correct before I generate the artifact?" --options "Looks correct,Request changes"
  human "Looks correct"
  perl -pi -e 's/^\[Answer\]:$/[Answer]: Looks correct/' "$q"; wrote "$q"
  step ok "$st: 要約確認の返答を記録" -- engine log answer --stage "$st" --checkpoint summary-confirmation \
    --questions-file "$q" --details "Looks correct"
}

# レビュー 1 回（stage-protocol-reviewer.md §12a Flow 1〜3）。
review_pass() {
  local st="$1" reviewer="$2" rf
  step ok "$st: レビュー依頼を記録" -- engine log review --stage "$st" --reviewer "$reviewer" --iteration 1
  rf="$(jq -r '.reviewFile // empty' "$LAST" 2>/dev/null)"
  check "$st: レビュー依頼が reviewFile を返す" "[ -n \"$rf\" ]"
  [ -n "$rf" ] || return 0
  mkdir -p "$(dirname "$rf")"
  cat > "$rf" <<EOF
## Review

**Verdict:** READY
**Reviewer:** $reviewer
**Date:** $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|

### Summary

問題なし。
EOF
  step ok "$st: レビュー結果を記録" -- engine log review --stage "$st" --reviewer "$reviewer" --iteration 1 --verdict READY
  local rec; rec="$(jq -r '.reviewRecord // empty' "$LAST" 2>/dev/null)"
  check "$st: レビュー記録が作られている" "[ -n \"$rec\" ] && [ -f \"$R/$rec\" ]"
}

# §13 の学びの儀式（候補 0 件でも「Anything to add?」は必ず聞く）。
learnings() {
  local st="$1" expect=ok
  # 最初のステージでは runtime-graph.json がまだ無い。2.8.2 は `orchestrate report` の後の
  # PostToolUse でしか runtime compile を発火させず（aidlc-lib.ts classifyRuntimeCompileCommand）、
  # 誕生から最初の承認までに report が無いため、2.8.2 自身も surface に失敗する。儀式は
  # 「advisory」（SKILL.md）なので失敗しても続行する。ここは数えない。
  [ "$st" = reverse-engineering ] && expect=any
  step $expect "$st: 学びの候補を出す" -- engine learnings surface --slug "$st"
  step ok "$st: 学びの質問を記録" -- engine log decision --stage "$st" --decision "Anything to add for next time?" --options "Nothing to add,Add a note"
  human "Nothing to add"
  step ok "$st: 学びの返答を記録" -- engine log answer --stage "$st" --details "Nothing to add"
}

# 承認ゲート。人間の返答が無いうちの承認は拒否されなければならない。
gate() {
  local st="$1"
  step ok "$st: 承認待ちを記録" -- engine orchestrate report --stage "$st" --result awaiting-approval
  step refuse "$st: 人間の返答なしの承認は拒否" -- engine orchestrate report --stage "$st" --result approved --user-input "Approve"
  human "Approve"
  step ok "$st: 承認" -- engine orchestrate report --stage "$st" --result approved --user-input "Approve"
}

# produces を仮の本文で埋める（questions ファイルは要約確認が書く）。
fill_produces() {
  local f
  for f in $(cur '.produces[] | select(endswith("-questions.md") | not)'); do
    printf '# %s\n\n仮の本文。\n\n## Sources\n\n- [desc]\n' "$(basename "$f")" | write "$f"
  done
}

DESC="statusline が空文字を出す不具合を直す"
R="aidlc/spaces/default/intents/260101-statusline-empty"

# --- 誕生 ------------------------------------------------------------------
step ok "誕生の print" -- engine orchestrate next --scope bugfix "$DESC"
step ok "intent を作る" -- engine intent create --scope bugfix --arguments="$DESC" --label "statusline empty"
R="aidlc/spaces/default/intents/$(cat aidlc/spaces/default/intents/active-intent 2>/dev/null)"
log "RECORD: $R"

# --- 2.1 reverse-engineering（pipeline） -------------------------------------
advance reverse-engineering
st=reverse-engineering; D="$R/inception/$st"
diary_check $st
fill_produces
printf '# developer scan\n\n仮。\n' | write "$D/developer-scan.md"
step refuse "$st: 順番外の link は拒否" -- engine log link --stage $st --link aidlc-architect-agent
step ok "$st: developer link" -- engine log link --stage $st --link aidlc-developer-agent --artifact "$D/developer-scan.md"
step ok "$st: architect link" -- engine log link --stage $st --link aidlc-architect-agent
learnings $st
gate $st

# --- 2.3 requirements-analysis（inline + advisory review） --------------------
advance requirements-analysis
st=requirements-analysis; D="$R/inception/$st"
diary_check $st
check "$st: consumes が実在するパスに解決されている" \
  "jq -e '[.consumes[] | select(test(\"\\\\.(md|json)$\") | not)] | length == 0' \"$OUT/current.json\" >/dev/null"
step refuse "$st: 要約確認の前の承認待ちは拒否" -- engine orchestrate report --stage $st --result awaiting-approval
summary_confirm $st "$D/requirements-analysis-questions.md"
printf '# 要件\n\n- FR-1: active intent が無いとき statusline は空文字でなく案内を出す\n\n## Sources\n\n- [desc]\n' | write "$D/requirements.md"
review_pass $st "$(cur .reviewer)"
learnings $st
gate $st

# --- 3.5 code-generation（subagent + plan approval + advisory review） --------
advance code-generation
st=code-generation; D="$R/construction/$st"
diary_check $st
check "$st: gate が決定済み（bugfix は skeleton なし）" "[ \"\$(cur .gate)\" = true ]"
{ printf '# Code Generation Plan\n\n'; aidlc engine testing-posture render
  printf '\n## Steps\n\n- [ ] Step 1: statusline を直す\n- [ ] Step 2: 回帰テストを書いて走らせる\n'; } > "$D/code-generation-plan.md"
wrote "$D/code-generation-plan.md"
printf '# Unit Test Instructions\n\n`cargo test -p aidlc --test statusline_contract`\n' | write "$D/unit-test-instructions.md"
step ok "$st: 承認指紋を出す" -- engine testing-posture fingerprint --stage-level
FP="$(cat "$LAST")"
check "$st: 指紋が 2 行タグ（Approval Fingerprint / Planned Source）" \
  "grep -q '^\[Approval Fingerprint\]: sha256:v3:' \"$LAST\" && grep -q '^\[Planned Source\]: ' \"$LAST\""
Q="$D/code-generation-questions.md"
printf '# Code Generation Questions\n\n## Plan Approval\n\nこの計画で進めてよいですか？\n\n%s\n\n- "Approve Plan" — proceed to code generation\n- "Request Changes" — revise the plan\n\n[Answer]:\n' "$FP" | write "$Q"
step ok "$st: 計画承認の提示を記録" -- engine log decision --stage $st --checkpoint plan-approval --session "$SESSION" \
  --questions-file "$Q" --decision "Approve this exact Code Generation plan?" --options "Approve Plan,Request Changes" --stage-level
human "Approve Plan"
perl -pi -e 's/^\[Answer\]:$/[Answer]: Approve Plan/' "$Q"; wrote "$Q"
step ok "$st: 計画承認の返答を記録" -- engine log answer --stage $st --checkpoint plan-approval --session "$SESSION" \
  --questions-file "$Q" --details "Approve Plan" --stage-level
step ok "$st: 開発者への指示書" -- engine testing-posture brief --stage-level
SRC=modules/app/aidlc/src/wording.rs
printf '\n// statusline fix (replay)\n' >> "$SRC"; wrote "$SRC"
perl -pi -e 's/- \[ \] Step/- [x] Step/' "$D/code-generation-plan.md"; wrote "$D/code-generation-plan.md"
printf '# Code Summary\n\n- %s を変更\n' "$SRC" | write "$D/code-summary.md"
printf '{"stage":"code-generation","unit":null,"version":1,"writes":[{"path":"%s"}]}\n' "$SRC" | write "$D/source-manifest.json"
printf '{"stage":"code-generation","upstream_ids":["FR-1"],"coverage":[{"id":"FR-1","status":"OK","target":"%s"}]}\n' "$SRC" | write "$D/traceability.json"
review_pass $st "$(cur .reviewer)"
learnings $st
gate $st

# --- 3.6 build-and-test / 4.1 deployment-pipeline / 4.3 deployment-execution ---
for st in build-and-test deployment-pipeline deployment-execution; do
  advance $st
  diary_check $st
  q="$(cur '.produces[] | select(endswith("-questions.md"))')"
  [ -n "$q" ] && summary_confirm $st "$q"
  fill_produces
  rv="$(cur '.reviewer // empty')"
  [ -n "$rv" ] && review_pass $st "$rv"
  learnings $st
  gate $st
done

step ok "完了" -- engine orchestrate next
check "ワークフローが done で終わる" "[ \"\$(jq -r .kind \"$LAST\")\" = done ]"

echo "replay: $((STEPS - FAILED))/$STEPS passed (transcript: $LOG)"
[ "$FAILED" -eq 0 ]
