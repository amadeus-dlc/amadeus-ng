#!/usr/bin/env bun
/** 同じ依頼の発行記録を別のプロジェクト位置へ複写した場合の本家の指紋。 */
import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { captureObservation } from "./capture-observation";
import { UPSTREAM, verifySource } from "./upstream-source";
const dist = process.argv[2], destination = process.argv[3];
assert(dist && destination);
verifySource(dist);
const corpus = JSON.parse(readFileSync(join(import.meta.dir, "../../tests/golden/upstream-a277af21/stage1/cases.json"), "utf8"));
assert.equal(corpus.source.commit, UPSTREAM.commit);
const original = corpus.observations.find((case_: { id: string }) => case_.id === "plan/decision");
assert(original);
const parent = mkdtempSync(join(tmpdir(), "aidlc-plan-project-binding-"));
const root = join(parent, "workspace");
try {
  cpSync(dist, root, { recursive: true });
  for (const [path, body] of Object.entries(original.initial_files) as [string, string][]) {
    if (!path.startsWith("aidlc/spaces/") && !path.startsWith("src/") && path !== ".gitignore") continue;
    assert(!path.split("/").includes(".."));
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), Buffer.from(body, "base64"));
  }
  const observation = captureObservation({ root, argv: [process.execPath, join(root, ".claude/tools/aidlc-testing-posture.ts"), "fingerprint", "--stage-level"] });
  assert.equal(observation.output.exit_code, 0);
  assert.equal(observation.output.stderr, "");
  const originalFingerprint = corpus.observations.find((case_: { id: string }) => case_.id === "testing-posture/fingerprint").output.stdout;
  assert.equal(observation.output.stdout, originalFingerprint);
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, fixture_origin: "upstream-a277af21/stage1/cases.json:plan/decision", fixture_kind: "固定元で成功した発行記録を異なるプロジェクト位置へ複写した合成入力", observations: [{ id: "copied-project-retains-fingerprint", ...observation }] }, null, 2) + "\n");
} finally { rmSync(parent, { recursive: true, force: true }); }
