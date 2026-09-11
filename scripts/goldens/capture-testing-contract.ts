#!/usr/bin/env bun
/** 固定本家が計画内のTesting Contractを受理する条件。 */
import assert from "node:assert/strict";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { UPSTREAM, verifySource } from "./upstream-source";
const dist = process.argv[2], destination = process.argv[3];
assert(dist && destination);
verifySource(dist);
const upstream = await import(pathToFileURL(join(dist, ".claude/tools/aidlc-testing-posture.ts")).href);
const contract = upstream.resolveTestingPostureFromSections({}, { scope: "bugfix", testStrategy: "minimal", projectType: "brownfield" });
const rendered = upstream.renderTestingContract(contract);
const { contract_sha256, ...withoutHash } = contract;
const inputs = [
  { id: "valid", plan: `# Plan\n\n${rendered}\n## Steps\n- [ ] Implement\n` },
  { id: "changed-methodology", plan: rendered.replace('"methodology": "test-after"', '"methodology": "tdd"') },
  { id: "version-2", plan: rendered.replace('"version": 1', '"version": 2') },
  { id: "version-one-as-float", plan: rendered.replace('"version": 1', '"version": 1.0') },
  { id: "fenced-heading-only", plan: `\`\`\`md\n${rendered}\n\`\`\`\n` },
  { id: "comment-containing-contract", plan: `<!--\n${rendered}\n-->\n` },
  { id: "missing-hash", plan: upstream.renderTestingContract(withoutHash) },
  { id: "uppercase-hash", plan: rendered.replace(contract_sha256, contract_sha256.toUpperCase()) },
  { id: "missing-contract-heading", plan: rendered.replace("## Testing Contract", "## Other") },
  { id: "invalid-json", plan: "## Testing Contract\n```json\n{broken}\n```\n" },
];
const observations = inputs.map(input => ({ ...input, plan_base64: Buffer.from(input.plan).toString("base64"), parsed: upstream.parseTestingContract(input.plan) }));
mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, JSON.stringify({ source: UPSTREAM, capture_command: [process.execPath, import.meta.path, dist, destination], tool_versions: { bun: Bun.version }, normalization: "none", observations }, null, 2) + "\n");
