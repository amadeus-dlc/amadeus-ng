#!/usr/bin/env bash
# 2 つの再生結果（replay-bugfix.sh の --out）の run-stage 指示をステージごとに突き合わせる。
#
#   scripts/aidlc-selfhost/e2e/compare-directives.sh <expected-out> <actual-out>
#
# 比べるのは指揮役の振る舞いを変えるキーだけである。規則本文（rules_content）と
# conductor_persona は load-steering の配送物なので比べない。intent の記録ディレクトリ名は
# 採番に日付を含むので `<record>` に置き換えてから比べる。差分があれば終了コード 1。
set -u
[ $# -eq 2 ] || { echo "usage: $0 <expected-out> <actual-out>" >&2; exit 2; }
EXP="$1" ACT="$2"
KEYS='{kind, stage, phase, lead_agent, support_agents, mode, gate, inline_context_paths, consumes, consumes_absent, produces, reviewer, review_artifact, review_class, reviewer_max_iterations, protocol_modules, sensors_applicable, pipeline, next_stage}'
DIFFS=0

normalize() {
  jq -S "$KEYS" "$1" | sed -E 's#intents/[0-9]{6}-[a-z0-9-]+/#intents/<record>/#g'
}

for f in "$EXP"/directives/*.json; do
  name="$(basename "$f")"
  if [ ! -f "$ACT/directives/$name" ]; then
    echo "== $name: actual has no directive"; DIFFS=$((DIFFS + 1)); continue
  fi
  if ! d="$(diff -u <(normalize "$f") <(normalize "$ACT/directives/$name"))"; then
    echo "== $name"; printf '%s\n' "$d" | tail -n +3; DIFFS=$((DIFFS + 1))
  fi
done
echo "compare: $DIFFS stage(s) differ"
[ "$DIFFS" -eq 0 ]
