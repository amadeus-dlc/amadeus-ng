#!/usr/bin/env bun
/** 固定本家へ不正UTF-8のstdinを渡したときの匿名HUMAN_TURN。 */
import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";
import { isolatedEnvironment } from "./capture-observation";
import { UPSTREAM, verifySource } from "./upstream-source";
const dist = process.argv[2], destination = process.argv[3];
assert(dist && destination);
verifySource(dist);
const parent = mkdtempSync(join(tmpdir(), "aidlc-human-utf8-"));
const root = join(parent, "workspace");
try {
  cpSync(dist, root, { recursive: true });
  const environment = isolatedEnvironment(root);
  const created = spawnSync(process.execPath, [join(root, ".claude/tools/aidlc-utility.ts"), "intent-create", "--scope", "bugfix", "--label", "utf8", "--arguments", "Check anonymous presence"], { cwd: root, env: environment });
  assert.equal(created.status, 0, created.stderr.toString());
  const intents = join(root, "aidlc/spaces/default/intents");
  const record = join(intents, readFileSync(join(intents, "active-intent"), "utf8").trim());
  const auditPath = join(record, "audit", readdirSync(join(record, "audit"))[0]);
  const before = readFileSync(auditPath);
  const state = readFileSync(join(record, "aidlc-state.md"));
  const argv = [process.execPath, join(root, ".claude/hooks/aidlc-record-human-turn.ts")];
  const input = Buffer.from([0xff]);
  const observed = spawnSync(argv[0], argv.slice(1), { cwd: root, env: environment, input });
  const after = readFileSync(auditPath);
  assert(after.subarray(0, before.length).equals(before));
  const appended = after.subarray(before.length);
  assert(appended.toString().includes("**Event**: HUMAN_TURN"));
  assert(readFileSync(join(record, "aidlc-state.md")).equals(state));
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, fixture_kind: "新しいbugfixの記録にbyte FFを渡す合成入力。保護されたsession/responseは無い", tool_versions: { bun: Bun.version }, observations: [{ id: "ff-anonymous-human-turn", input: { argv, stdin_base64: input.toString("base64"), environment }, output: { exit_code: observed.status, stdout_base64: observed.stdout.toString("base64"), stderr_base64: observed.stderr.toString("base64") }, appended_audit_base64: appended.toString("base64"), state_unchanged: true }] }, null, 2) + "\n");
} finally { rmSync(parent, { recursive: true, force: true }); }
