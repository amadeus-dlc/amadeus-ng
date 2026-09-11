#!/usr/bin/env bun
import assert from "node:assert/strict";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { captureDoctor } from "./capture-doctor";
import { captureLearnings } from "./capture-learnings";

const [dist, output] = process.argv.slice(2);
assert(dist && output, "Usage: bun capture-extensions.ts <verified-dist> <corpus-dir>");
for (const [name, capture] of [["doctor", captureDoctor], ["learnings", captureLearnings]] as const) {
  const result = capture(dist);
  const target = join(output, name);
  mkdirSync(target, { recursive: true });
  writeFileSync(join(target, "cases.json"), JSON.stringify(result, null, 2) + "\n");
  console.log(`${name}: ${result.observations.length} 観測`);
}
