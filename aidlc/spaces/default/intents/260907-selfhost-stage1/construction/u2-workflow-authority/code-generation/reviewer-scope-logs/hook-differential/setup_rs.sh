#!/bin/bash
# Rust 側の一時ワークスペース (契約テストの Workspace::new と同じ手順)。
set -e
D=/private/tmp/claude-501/-Users-j5ik2o-orca-workspaces-amadeus-ng-stage1/8f466f39-1de4-4b36-90eb-8d6e51b03145/scratchpad/diff
REPO=/Users/j5ik2o/orca/workspaces/amadeus-ng/stage1
R=$D/rproj
rm -rf "$R"; mkdir -p "$R/workspace/.claude/tools/data" "$R/workspace/.claude/scopes" "$R/workspace/aidlc/spaces/default/intents" "$R/home"
for n in stage-graph.json scope-grid.json harness.json; do cp "$REPO/tests/golden/upstream-a277af21/data/$n" "$R/workspace/.claude/tools/data/$n"; done
cp "$REPO/.claude/scopes/aidlc-bugfix.md" "$R/workspace/.claude/scopes/aidlc-bugfix.md"
for t in aidlc-utility aidlc-log aidlc; do cp "$REPO/target/debug/aidlc" "$R/$t"; done
echo 'fn main() {}' > "$R/workspace/source.rs"
cd "$R/workspace"
env -i HOME="$R/home" PATH=/usr/bin:/bin "$R/aidlc-utility" intent-create --scope bugfix --label guards --arguments "Fix the review guards"
REC=$(cat "$R/workspace/aidlc/spaces/default/intents/active-intent")
echo "record=$REC"
ls "$R/workspace/aidlc/spaces/default/intents/$REC/audit"
