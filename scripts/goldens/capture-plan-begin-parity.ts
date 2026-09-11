#!/usr/bin/env bun
/** 同じ一時workspaceでnativeと固定本家の実装開始を実行し、全バイトを比較する。 */
import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdirSync, mkdtempSync, rmSync, existsSync } from "node:fs";
import { resolve, join, dirname } from "node:path";
import { tmpdir } from "node:os";
import { createHash } from "node:crypto";
import { UPSTREAM, verifySource } from "./upstream-source";
const [dist, destination] = process.argv.slice(2);
assert(dist && destination, "固定distと出力先が必要");
verifySource(dist);
const root = resolve(import.meta.dir, "../..");
const temporary = mkdtempSync(join(tmpdir(), "aidlc-begin-parity-"));
const evidence = join(temporary, "observed.json");
try {
  const command = ["cargo", "test", "-p", "aidlc", "--test", "upstream_271_contract", "begin_publishes_generation", "--", "--nocapture"];
  const result = Bun.spawnSync(command, { cwd: root, env: { ...process.env, AIDLC_PLAN_BEGIN_CAPTURE: evidence, AIDLC_PLAN_DECISION_SOURCE: resolve(dist), AIDLC_PLAN_DECISION_BUN: process.execPath }, stdout: "inherit", stderr: "inherit" });
  assert(existsSync(evidence), `比較結果の採取前に終了した: ${result.exitCode}`);
  const observed = JSON.parse(readFileSync(evidence, "utf8"));
  if (result.exitCode !== 0) {
    writeFileSync("/tmp/amadeus-u2-plan-begin-parity-failure.json", JSON.stringify(observed, null, 2) + "\n");
    throw new Error("同一入力の比較が失敗。/tmp/amadeus-u2-plan-begin-parity-failure.json を参照");
  }
  assert.equal(observed.native.receipt_base64, observed.upstream.receipt_base64);
  const binary = join(root, "target/debug/aidlc");
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({
    source: UPSTREAM,
    capture_command: [process.execPath, import.meta.path, dist, destination],
    native_binary_sha256: createHash("sha256").update(readFileSync(binary)).digest("hex"),
    tool_versions: { bun: Bun.version },
    comparison: { receipt: "exact bytes", normalization: [], scope: "実際のbugfix進行でCode Generationへ到達した同じworkspace。同じ承認文書を保持し、native開始直前のapproved受領を復元して本家beginへ渡す。受領の全バイト・stdout/stderrを比較する。" },
    observations: [observed],
  }, null, 2) + "\n");
} finally { rmSync(temporary, { recursive: true, force: true }); }
