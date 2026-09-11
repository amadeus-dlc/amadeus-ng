#!/usr/bin/env bun
/** 固定本家のlog拒否を実CLIで採り、元のstdout/stderr/auditを保存する。 */
import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { UPSTREAM, verifySource } from "./upstream-source";
import { captureObservation } from "./capture-observation";

const [dist, destination] = process.argv.slice(2);
assert(dist && destination);
verifySource(dist);
const root = mkdtempSync(join(tmpdir(), "aidlc-log-failure-"));
try {
  cpSync(dist, root, { recursive: true });
  const invoke = (args: string[]) => captureObservation({ root, argv: [process.execPath, join(root, ".claude/tools/aidlc-log.ts"), ...args, "--project-dir", root] });
  const observations = [{ id: "cold-decision", ...invoke(["decision", "--stage", "domain-design"]) }];
  const setup = captureObservation({ root, argv: [process.execPath, join(root, ".claude/tools/aidlc-utility.ts"), "intent-create", "--scope", "bugfix", "--label", "failure"] });
  assert.equal(setup.output.exit_code, 0, setup.output.stderr);
  observations.push({ id: "setup", ...setup });
  for (const [id, args] of [
    ["decision", ["decision", "--stage", "domain-design"]],
    ["repeat-decision", ["decision", "--stage", "domain-design"]],
    ["review", ["review", "--stage", "domain-design"]],
    ["unknown", ["unknown"]],
  ] as const) {
    const observed = invoke([...args]);
    assert.equal(observed.output.exit_code, 1);
    assert.equal(observed.output.stdout, "");
    assert.equal(typeof JSON.parse(observed.output.stderr).error, "string");
    observations.push({ id, ...observed });
  }
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, normalization: [], capture_method: "検証済み固定配布のaidlc-log.tsを別プロセスで実行。cold、作業開始、同一拒否の反復、review文法、未知動詞を順に採取。既存コーパスは変更しない。", observations }, null, 2) + "\n");
} finally { rmSync(root, { recursive: true, force: true }); }
