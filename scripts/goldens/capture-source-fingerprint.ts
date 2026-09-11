#!/usr/bin/env bun
/** 固定本家のワークスペースソース指紋。U2の計画承認の参照観測。 */
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { pathToFileURL } from "node:url";
import { UPSTREAM, verifySource } from "./upstream-source";
const dist = process.argv[2], destination = process.argv[3];
assert(dist && destination);
verifySource(dist);
const upstream = await import(pathToFileURL(join(dist, ".claude/tools/aidlc-lib.ts")).href);
const files = {
  "src/lib.rs": "pub fn example() {}\n",
  "README.md": "# Example\n",
  "target/output.txt": "generated output\n",
  "aidlc/spaces/default/memory/org.md": "# Rules\n",
  ".claude/tools/data/harness.json": "{\"name\":\"claude\"}\n",
  ".claude/tools/local.ts": "export const harness = true;\n",
};
const observations = [];
for (const [id, inputs] of [["regular-files-and-shell-exclusions", files], ["empty-workspace", {}]] as const) {
  const root = mkdtempSync(join(tmpdir(), "aidlc-source-fingerprint-"));
  try {
    for (const [path, body] of Object.entries(inputs)) { mkdirSync(dirname(join(root, path)), { recursive: true }); writeFileSync(join(root, path), body); }
    const fingerprint = upstream.workspaceSourceFingerprint(root);
    assert.equal(typeof fingerprint, "string");
    observations.push({ id, files: inputs, fingerprint });
  } finally { rmSync(root, { recursive: true, force: true }); }
}
mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, JSON.stringify({ source: UPSTREAM, capture_command: [process.execPath, import.meta.path, dist, destination], tool_versions: { bun: Bun.version }, normalization: "none", observations }, null, 2) + "\n");
