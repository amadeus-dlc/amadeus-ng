#!/bin/bash
# classic scope の採取列の先頭を native で流し、tests/golden/upstream-a277af21/cli と突き合わせる。
# 主セッションで実行すること（委任エージェントは state-transition-guard に拒否される）。
# 前提: bun / jq / python3、vendor/aidlc-workflows に a277af21 が fetch 済み、target/debug/aidlc がビルド済み。
set -u
REPO=$(cd "$(dirname "$0")/../../../../../../../../../.." && pwd)
HERE=$(cd "$(dirname "$0")" && pwd)
WORK=${AIDLC_STEP8_PROBE_WORK:-$(mktemp -d "${TMPDIR:-/tmp}/aidlc-step8-probe-XXXXXX")}
DIST=$WORK/dist271/dist/claude
CORPUS=$REPO/tests/golden/upstream-a277af21
LOGS=$HERE/out
rm -rf "$LOGS"; mkdir -p "$LOGS" "$WORK/dist271"
cd "$REPO"
git -C vendor/aidlc-workflows archive a277af218f0df7f325d3b8be7b6d90fce2c5bd40 dist/claude | tar -x -C "$WORK/dist271"
{
echo "# native classic-scope probe $(date -u +%FT%TZ)"
echo "binary: target/debug/aidlc sha256=$(shasum -a 256 target/debug/aidlc | cut -c1-16) mtime=$(ls -la --time-style=full-iso target/debug/aidlc | awk '{print $6" "$7}')"
echo "dist: git archive a277af218f0df7f325d3b8be7b6d90fce2c5bd40 dist/claude (vendor/aidlc-workflows) -> $DIST"
bun -e 'import {verifySource} from "./scripts/goldens/upstream-source"; const r = verifySource(process.argv[1]); console.log("verifySource: ok commit="+r.commit+" files="+r.files+" manifest="+r.manifest)' "$DIST" 2>&1
} > "$LOGS/README.txt"
W=$WORK/native-ws; rm -rf "$W"; mkdir -p "$W"
# 本家採取の makeWorkspace（scripts/goldens/capture-cli.ts:202-214）と同じく `.claude` と `aidlc` だけを置く。
cp -R "$DIST/.claude" "$W/.claude"
cp -R "$DIST/aidlc" "$W/aidlc"
# バイナリと HOME は作業ツリーの外へ置く（本家採取時の作業ツリーには配布シェルだけがあり、走査対象に余分なファイルを混ぜないため）。
BIN=$WORK/bin; CAPHOME=$WORK/capture-home
mkdir -p "$BIN" "$CAPHOME"
for n in aidlc-orchestrate aidlc-utility aidlc-jump aidlc-state; do cp target/debug/aidlc "$BIN/$n"; done
E=(env -i PATH=/usr/bin:/bin HOME="$CAPHOME" LANG=C.UTF-8 LC_ALL=C.UTF-8 TZ=UTC CLAUDE_PROJECT_DIR="$W" AIDLC_PROJECT_DIR="$W" AIDLC_SKIP_HUMAN_PRESENCE_GUARD=1 AIDLC_DISABLE_ENSEMBLE_EVIDENCE=1 AIDLC_SKIP_SUMMARY_CONFIRMATION_GUARD=1 AIDLC_SKIP_ARTIFACT_GUARD=1 AIDLC_DISABLE_USAGE_TRACKING=1)
run() { local name=$1 tool=$2; shift 2; (cd "$W" && "${E[@]}" "$BIN/$tool" "$@" --project-dir "$W") > "$LOGS/$name.stdout" 2> "$LOGS/$name.stderr"; echo $? > "$LOGS/$name.exit"; echo "$name: exit $(cat "$LOGS/$name.exit") stdout=$(wc -c < "$LOGS/$name.stdout")B stderr=$(wc -c < "$LOGS/$name.stderr")B" >> "$LOGS/README.txt"; }
snap() { local tag=$1; local rec="$W/aidlc/spaces/default/intents/$(cat "$W/aidlc/spaces/default/intents/active-intent" 2>/dev/null)"; cp "$rec/aidlc-state.md" "$LOGS/$tag.state.md" 2>/dev/null; cat "$rec"/audit/*.md > "$LOGS/$tag.audit.md" 2>/dev/null; ls "$rec"/audit/ > "$LOGS/$tag.audit-shards.txt" 2>/dev/null; }
run next-no-active-intent aidlc-orchestrate next --scope classic
run intent-create aidlc-utility intent-create --scope classic --label golden --arguments "Build a small ordering service"
snap after-intent-create
run next-start aidlc-orchestrate next
snap after-next-start
TOKEN=$(jq -r '.continue_token // empty' "$LOGS/next-start.stdout")
echo "token length: ${#TOKEN}" >> "$LOGS/README.txt"
run continue-load-steering aidlc-orchestrate continue "$TOKEN"
snap after-continue
run next-stage-jump-print aidlc-orchestrate next --stage contract-design
run jump-resolve aidlc-jump resolve --stage domain-design
CLONE=$(basename "$(ls "$W"/aidlc/spaces/default/intents/*/audit/*.md 2>/dev/null | head -1)" .md)
echo "clone shard: $CLONE host: $(hostname)" >> "$LOGS/README.txt"
bun "$HERE/compare.ts" "$LOGS" "$CORPUS" "$W" "$CLONE" "$(hostname)" > "$LOGS/compare.txt" 2>&1
norm() { sed -e "s#$(realpath "$W")#<ROOT>#g" -e "s#$W#<ROOT>#g" -e "s#$CLONE#<CLONE>#g" -e "s#$(hostname)#<CLONE>#g" -E -e 's/[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?Z/<TS>/g' -e 's#intents/[0-9]{6}-#intents/<TS>-#g' -e 's/[0-9]{6}-golden/<TS>-golden/g'; }
norm < "$LOGS/after-intent-create.state.md" > "$LOGS/after-intent-create.state.norm.md"
diff -u "$CORPUS/cli/intent-create/classic-scope/state-full.md" "$LOGS/after-intent-create.state.norm.md" > "$LOGS/state-full.diff"; echo "state-full diff exit: $? (0 = identical)" >> "$LOGS/README.txt"
norm < "$LOGS/after-intent-create.audit.md" > "$LOGS/after-intent-create.audit.norm.md"
diff -u "$CORPUS/cli/intent-create/classic-scope/audit.md" "$LOGS/after-intent-create.audit.norm.md" > "$LOGS/genesis-audit.diff"; echo "genesis audit diff exit: $? (0 = identical)" >> "$LOGS/README.txt"
diff -u "$LOGS/after-intent-create.state.md" "$LOGS/after-next-start.state.md" > "$LOGS/next-start.native-state.diff"; echo "native state change on next: $(wc -l < "$LOGS/next-start.native-state.diff") lines (corpus next/start/state.diff has $(wc -l < "$CORPUS/cli/next/start/state.diff") lines)" >> "$LOGS/README.txt"
python3 - "$LOGS/after-intent-create.audit.md" "$LOGS/after-next-start.audit.md" <<'PY' | norm > "$LOGS/next-start.native-audit-delta.norm.md"
import sys
a=open(sys.argv[1],'rb').read(); b=open(sys.argv[2],'rb').read()
sys.stdout.buffer.write(b[len(a):] if b.startswith(a) else b)
PY
diff -u "$CORPUS/cli/next/start/audit.md" "$LOGS/next-start.native-audit-delta.norm.md" > "$LOGS/next-start.audit.diff"; echo "next/start audit delta diff exit: $? (corpus audit.md $(wc -c < "$CORPUS/cli/next/start/audit.md")B)" >> "$LOGS/README.txt"
echo "DONE" >> "$LOGS/README.txt"
cat "$LOGS/README.txt"; echo "===== compare.txt"; cat "$LOGS/compare.txt"; echo "===== state-full.diff (head 60)"; head -60 "$LOGS/state-full.diff"; echo "===== genesis-audit.diff (head 40)"; head -40 "$LOGS/genesis-audit.diff"; echo "===== next-start audit diff (head 30)"; head -30 "$LOGS/next-start.audit.diff"
