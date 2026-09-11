#!/usr/bin/env bash
# 着地確認（closeout）の差分駆動に使う一時ワークスペース群を CLOSEOUT_ROOT の下に組む。
#
# - 本家は固定コミット a277af21 の実バイトを `git archive` で CLOSEOUT_ROOT/pinned へ展開し、
#   各ワークスペースの `.claude/` はその写しである（本リポジトリの `.claude/hooks/` は使わない）。
# - memory 層は本リポジトリの `aidlc/spaces/default/memory/` の写し（バイト同一）なので、
#   前担当が本リポジトリを project dir にして採った束のダイジェスト（b503b8…）がそのまま
#   照合の第 3 の証人になる。
# - 本リポジトリには何も書かない。CLOSEOUT_ROOT 配下だけを作り直す。
#
# 使い方: CLOSEOUT_ROOT=<dir> bash closeout-workspaces.sh   （本リポジトリ根で実行する）
set -euo pipefail
: "${CLOSEOUT_ROOT:?CLOSEOUT_ROOT is required}"
REPO="${REPO:-$PWD}"
LOGS="$REPO/aidlc/spaces/default/intents/260907-selfhost-stage1/construction/u2-workflow-authority/code-generation/stage-rules-logs"
PIN=a277af218f0df7f325d3b8be7b6d90fce2c5bd40
ROOT="$CLOSEOUT_ROOT"

mkdir -p "$ROOT"
for d in pinned home ws stateless norules norules-with-harness fixture bom bom2 ctrl tabstage nospace nostage dangling boundary duplicate; do
  rm -rf "$ROOT/$d"
done
mkdir -p "$ROOT/pinned" "$ROOT/home"

# 固定コミットの実バイト。
git -C "$REPO/vendor/aidlc-workflows" archive "$PIN" dist/claude | tar -x -C "$ROOT/pinned"
PINNED="$ROOT/pinned/dist/claude"
{
  echo "# pinned commit: $PIN"
  (cd "$PINNED/.claude" && shasum -a 256 \
    hooks/aidlc-deliver-stage-rules.ts tools/aidlc-steering.ts tools/aidlc-graph.ts \
    tools/aidlc-lib.ts tools/aidlc-runtime-paths.ts tools/data/stage-graph.json tools/data/harness.json)
} > "$ROOT/pinned.sha256"

harness() {
  mkdir -p "$1"
  cp -R "$PINNED/.claude" "$1/.claude"
}
memory_from_repo() {
  mkdir -p "$1/aidlc/spaces/default"
  cp -R "$REPO/aidlc/spaces/default/memory" "$1/aidlc/spaces/default/memory"
  printf 'default\n' > "$1/aidlc/active-space"
}
memory_from_fixture() {
  mkdir -p "$1/aidlc/spaces/default"
  cp -R "$LOGS/fixture-memory" "$1/aidlc/spaces/default/memory"
}
# $1 = ws, $2 = Current Status 節に置く 1 行（状態ファイルはフックが読むだけ）。
record() {
  mkdir -p "$1/aidlc/spaces/default/intents/rec-0001"
  printf 'rec-0001\n' > "$1/aidlc/spaces/default/intents/active-intent"
  printf '# AI-DLC State\n\n## Current Status\n%s\n' "$2" \
    > "$1/aidlc/spaces/default/intents/rec-0001/aidlc-state.md"
}

# ws — 本リポジトリ相当。memory は本リポジトリの写し、Current Stage は code-generation。
harness "$ROOT/ws"; memory_from_repo "$ROOT/ws"
record "$ROOT/ws" '- **Current Stage**: code-generation'

# stateless — 固定コミットの dist/claude そのもの（.claude と配布 memory 雛形。記録なし）。
#             前担当 53〜60 の <repo>/vendor/aidlc-workflows/dist/claude に相当。
mkdir -p "$ROOT/stateless"; cp -R "$PINNED/." "$ROOT/stateless/"

# norules — 何も無いディレクトリ。前担当 52 の <repo>/modules に相当（.claude も aidlc も無い）。
mkdir -p "$ROOT/norules"

# norules-with-harness — .claude はあるが aidlc/（規則）が無い。52 の双子で、配布形の差を切り分ける。
harness "$ROOT/norules-with-harness"

# fixture — 前担当の合成 fixture（upstream-fixture-block.txt の材料）。記録なし。
harness "$ROOT/fixture"; memory_from_fixture "$ROOT/fixture"

# bom — fixture の org.md の先頭に UTF-8 BOM（EF BB BF）を足したもの。
harness "$ROOT/bom"; memory_from_fixture "$ROOT/bom"
{ printf '\357\273\277'; cat "$LOGS/fixture-memory/org.md"; } > "$ROOT/bom/aidlc/spaces/default/memory/org.md"

# bom2 — fixture の org.md の先頭に UTF-8 BOM を 2 つ並べたもの（落ちるのは先頭の 1 つだけか）。
harness "$ROOT/bom2"; memory_from_fixture "$ROOT/bom2"
{ printf '\357\273\277\357\273\277'; cat "$LOGS/fixture-memory/org.md"; } > "$ROOT/bom2/aidlc/spaces/default/memory/org.md"

# ctrl — fixture の project.md を制御文字入りに差し替えたもの（JSON エスケープの一致を見る）。
harness "$ROOT/ctrl"; memory_from_fixture "$ROOT/ctrl"
printf '# Project\n\nctrl:\001\037\177 nbsp:\302\240 ls:\342\200\250 ps:\342\200\251 nul:\000 tab:\t cr:\r\nquote:" backslash:\\ slash:/ lt:<script> unicode:\360\237\230\200\n' \
  > "$ROOT/ctrl/aidlc/spaces/default/memory/project.md"

# tabstage — ws と同じだが、状態ファイルの Current Stage がタブ区切り。
harness "$ROOT/tabstage"; memory_from_repo "$ROOT/tabstage"
record "$ROOT/tabstage" "$(printf -- '- **Current Stage**:\tcode-generation')"

# nospace — ws と同じだが、Current Stage の後ろに空白が無い。
harness "$ROOT/nospace"; memory_from_repo "$ROOT/nospace"
record "$ROOT/nospace" '- **Current Stage**:code-generation'

# nostage — 記録はあるが Current Stage 行が無い。
harness "$ROOT/nostage"; memory_from_repo "$ROOT/nostage"
record "$ROOT/nostage" '- **Something Else**: value'

# dangling — 記録 rec-0001 は在るのに active-intent カーソルが無い記録名 gone を指す。
harness "$ROOT/dangling"; memory_from_repo "$ROOT/dangling"
record "$ROOT/dangling" '- **Current Stage**: code-generation'
printf 'gone\n' > "$ROOT/dangling/aidlc/spaces/default/intents/active-intent"

# boundary / duplicate — 各駆動スクリプトが自分で中身を組む。
harness "$ROOT/boundary"
mkdir -p "$ROOT/boundary/aidlc/spaces/default/memory/phases"
printf '# Team\n' > "$ROOT/boundary/aidlc/spaces/default/memory/team.md"
printf '# Project\n' > "$ROOT/boundary/aidlc/spaces/default/memory/project.md"
printf '# Construction\n' > "$ROOT/boundary/aidlc/spaces/default/memory/phases/construction.md"

harness "$ROOT/duplicate"; memory_from_repo "$ROOT/duplicate"
record "$ROOT/duplicate" '- **Current Stage**: code-generation'

echo "workspaces ready under $ROOT"
cat "$ROOT/pinned.sha256"
