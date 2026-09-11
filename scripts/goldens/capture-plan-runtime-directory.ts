#!/usr/bin/env bun
/** 固定本家の受領除去と全体リセットが残すディレクトリを区別する。 */
import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdirSync, mkdtempSync, rmSync, existsSync, readdirSync } from "node:fs";
import { resolve, join, dirname } from "node:path";
import { tmpdir } from "node:os";
import { pathToFileURL } from "node:url";
import { UPSTREAM, verifySource } from "./upstream-source";
const [dist, destination] = process.argv.slice(2);
assert(dist && destination, "固定distと出力先が必要");
verifySource(dist);
const source = await import(pathToFileURL(join(resolve(dist), ".claude/tools/aidlc-lib.ts")).href);
const corpus = JSON.parse(readFileSync(join(import.meta.dir, "../../tests/golden/selfhost-stage1/plan-readiness.json"), "utf8"));
const receipt = corpus.observations[0].receipt;
const temporary = mkdtempSync(join(tmpdir(), "aidlc-plan-directory-"));
const project = join(temporary, "project");
mkdirSync(join(project, "aidlc"), { recursive: true });
const directory = join(project, "aidlc/.aidlc-sessions/plan-approval");
const observed = (id: string) => ({ id, directory_exists: existsSync(directory), files: existsSync(directory) ? readdirSync(directory).sort() : [] });
try {
  const observations = [observed("uninitialized")];
  source.writePlanApprovalReceipt(project, receipt);
  observations.push(observed("receipt-written"));
  source.clearPlanApprovalReceipt(project, receipt);
  observations.push(observed("receipt-cleared"));
  source.writePlanApprovalReceipt(project, receipt);
  source.resetPlanApprovalRuntime(project);
  observations.push(observed("runtime-reset"));
  assert.equal(observations[2].directory_exists, true);
  assert.equal(observations[3].directory_exists, false);
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, capture_command: process.argv, input: { receipt }, normalization: [], observations }, null, 2) + "\n");
} finally { rmSync(temporary, { recursive: true, force: true }); }
