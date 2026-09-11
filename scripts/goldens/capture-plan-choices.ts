#!/usr/bin/env bun
/** 固定本家の保護された選択肢への応答照合。 */
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { pathToFileURL } from "node:url";
import { UPSTREAM, verifySource } from "./upstream-source";
const dist = process.argv[2], destination = process.argv[3];
assert(dist && destination);
verifySource(dist);
const upstream = await import(pathToFileURL(join(dist, ".claude/tools/aidlc-testing-posture.ts")).href);
const lib = await import(pathToFileURL(join(dist, ".claude/tools/aidlc-lib.ts")).href);
const corpus = JSON.parse(readFileSync(join(import.meta.dir, "../../tests/golden/upstream-a277af21/stage1/cases.json"), "utf8"));
const decision = corpus.observations.find((case_: { id: string }) => case_.id === "plan/decision");
const encoded = Object.entries(decision.changed_files).find(([path]) => path.endsWith("challenge-stage1-session.json"))![1] as string;
const challenge = JSON.parse(Buffer.from(encoded, "base64").toString("utf8"));
const evidence = { authority: { ...challenge, unit: null, stageDir: "unused" }, fingerprint: challenge.fingerprint, questionsRelativePath: challenge.questionsFile, questionsPath: "unused", questionsSha256: "0".repeat(64), promptSha256: challenge.promptSha256 };
const inputs = [
  { id: "canonical-approve", response: "Approve Plan", exact: false },
  { id: "canonical-changes", response: "Request Changes", exact: false },
  { id: "number-one", response: "1", exact: false },
  { id: "number-two", response: "2", exact: false },
  { id: "number-bom", response: "\uFEFF1", exact: false },
  { id: "number-nel", response: "\u00851", exact: false },
  { id: "exact-disallows-number", response: "1", exact: true },
  { id: "exact-allows-label", response: "approve plan", exact: true },
  { id: "unrelated-text", response: "continue", exact: false },
];
const root = mkdtempSync(join(tmpdir(), "aidlc-plan-choices-"));
const observations = [];
try {
  for (const input of inputs) {
    const issued = upstream.recordPlanApprovalChallenge(root, evidence, challenge.session, challenge.options, input.exact);
    const result = upstream.recordPlanApprovalHumanResponse(root, challenge.session, input.response);
    const response = lib.readPlanApprovalResponse(root, challenge.session);
    observations.push({ ...input, options: challenge.options, session: challenge.session, challenge_id: issued.challengeId, recorded: result.recorded, choice: response?.choice ?? null, response_sha256: response?.responseSha256 ?? null });
  }
} finally { rmSync(root, { recursive: true, force: true }); }
mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, JSON.stringify({ source: UPSTREAM, fixture_origin: "upstream-a277af21/stage1/cases.json:plan/decision", capture_command: [process.execPath, import.meta.path, dist, destination], tool_versions: { bun: Bun.version }, observations }, null, 2) + "\n");
