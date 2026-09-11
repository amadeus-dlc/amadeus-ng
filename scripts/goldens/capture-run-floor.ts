#!/usr/bin/env bun
/** 固定本家のコード生成実行境界の識別。 */
import assert from "node:assert/strict";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { UPSTREAM, verifySource } from "./upstream-source";
const dist = process.argv[2], destination = process.argv[3];
assert(dist && destination);
verifySource(dist);
const upstream = await import(pathToFileURL(join(dist, ".claude/tools/aidlc-lib.ts")).href);
const cases = [
  { id: "unstarted", events: [] },
  { id: "workflow-started", events: ["WORKFLOW_STARTED"] },
  { id: "stage-started", events: ["WORKFLOW_STARTED", "STAGE_STARTED"] },
  { id: "rejected", events: ["WORKFLOW_STARTED", "STAGE_STARTED", "GATE_REJECTED"] },
  { id: "second-attempt", events: ["WORKFLOW_STARTED", "STAGE_STARTED", "GATE_REJECTED", "STAGE_STARTED"] },
  { id: "jump-boundary", events: ["WORKFLOW_STARTED", "STAGE_STARTED", "STAGE_JUMPED", "STAGE_STARTED"] },
];
const observations = cases.map(input => {
  const rows = input.events.map((event, index) => ({ event, timestamp: `2026-09-08T01:00:0${index}Z`, shard: "capture.md", shardIndex: 0, pos: index, block: `**Stage**: code-generation\n` }));
  const floor = upstream.latestMainWorkflowStageRunFloorForProject("unused", "code-generation", false, undefined, rows);
  return { id: input.id, events: rows, floor };
});
mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, JSON.stringify({ source: UPSTREAM, capture_command: [process.execPath, import.meta.path, dist, destination], tool_versions: { bun: Bun.version }, normalization: "none", observations }, null, 2) + "\n");
