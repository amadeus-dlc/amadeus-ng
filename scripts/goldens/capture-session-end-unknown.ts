#!/usr/bin/env bun
/** 未知のsession stampを持つ終了通知の、heartbeatなしdropを採取する。 */
import assert from "node:assert/strict";
import { cpSync, mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { UPSTREAM, verifySource } from "./upstream-source";
import { captureObservation } from "./capture-observation";
const [dist, destination] = process.argv.slice(2);
assert(dist && destination); verifySource(dist);
const temporary = mkdtempSync(join(tmpdir(), "aidlc-session-unknown-"));
const root = join(temporary, "workspace");
try {
  cpSync(dist, root, { recursive: true });
  mkdirSync(join(root, "src")); writeFileSync(join(root, "src/lib.rs"), "pub fn smoke() {}\n");
  const created = captureObservation({ root, argv: [process.execPath, join(root, ".claude/tools/aidlc-utility.ts"), "intent-create", "--scope", "bugfix", "--label", "session-unknown", "--arguments", "Verify session attribution"] });
  assert.equal(created.output.exit_code, 0);
  mkdirSync(join(root, "aidlc/.aidlc-sessions"), { recursive: true });
  writeFileSync(join(root, "aidlc/.aidlc-sessions/session-a"), "0191aaaa-bbbb-7ccc-9ddd-eeeeffff0001\n");
  const observed = captureObservation({ root, argv: [process.execPath, join(root, ".claude/hooks/aidlc-session-end.ts")], stdin: '{"session_id":"session-a","reason":"exit"}' });
  assert.equal(observed.output.exit_code, 0);
  assert(Object.keys(observed.changed_files).some(path => path.endsWith("session-end.drops")));
  assert(!Object.keys(observed.changed_files).some(path => path.endsWith("session-end.last")));
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, capture_method: "固定元の公開intent-createの後、session stampだけを未知UUIDへ置く。公開SessionEndの出力と全変更を無加工保存。", normalization: [], observations: [{ id: "unknown-stamp", ...observed }] }, null, 2) + "\n");
} finally { rmSync(temporary, { recursive: true, force: true }); }
