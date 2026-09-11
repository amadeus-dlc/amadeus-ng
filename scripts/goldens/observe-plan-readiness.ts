#!/usr/bin/env bun
/** 一時fixture内の本家2.7.1だけで、実装開始判定の入力と全出力を採取する。 */
import assert from "node:assert/strict";
import { readFileSync, writeFileSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { UPSTREAM, verifySource } from "./upstream-source";
const [dist, project, questions, receiptPath, destination] = process.argv.slice(2);
assert(dist && project && questions && receiptPath && destination, "dist/project/questions/receipt/destinationが必要");
verifySource(dist);
const posture = await import(pathToFileURL(join(resolve(dist), ".claude/tools/aidlc-testing-posture.ts")).href);
const lib = await import(pathToFileURL(join(resolve(dist), ".claude/tools/aidlc-lib.ts")).href);
const questionPath = join(project, questions);
const paths = {
  plan: join(dirname(questionPath), "code-generation-plan.md"),
  instructions: join(dirname(questionPath), "unit-test-instructions.md"),
  questions: questionPath,
  receipt: receiptPath,
  team: join(project, "aidlc/spaces/default/memory/team.md"),
  source: join(project, "readiness-edit.rs"),
};
const before = new Map(Object.values(paths).map(path => [path, existsSync(path) ? readFileSync(path) : null]));
const restore = () => { for (const [path, bytes] of before) { if (bytes === null) rmSync(path, { force: true }); else { mkdirSync(dirname(path), { recursive: true }); writeFileSync(path, bytes); } } };
const receipt = JSON.parse(readFileSync(paths.receipt, "utf8"));
const authority = posture.resolveCodeGenerationAuthority(project, { unit: null });
const observations: unknown[] = [];
const cases: [string, () => void][] = [
  ["approved", () => {}],
  ["missing-receipt", () => rmSync(paths.receipt)],
  ["changed-source-before-generation", () => writeFileSync(paths.source, "fn changed() {}\n")],
  ["generation-with-changed-source", () => { writeFileSync(paths.receipt, JSON.stringify({ ...receipt, status: "generation" }, null, 2) + "\n"); writeFileSync(paths.source, "fn changed() {}\n"); }],
  ["generation-with-changed-plan", () => { writeFileSync(paths.receipt, JSON.stringify({ ...receipt, status: "generation" }, null, 2) + "\n"); writeFileSync(paths.plan, readFileSync(paths.plan,"utf8") + "Changed plan\n"); }],
  ["generation-with-changed-questions", () => { writeFileSync(paths.receipt, JSON.stringify({ ...receipt, status: "generation" }, null, 2) + "\n"); writeFileSync(paths.questions, readFileSync(paths.questions,"utf8") + "\n"); }],
  ["generation-with-changed-current-contract", () => { writeFileSync(paths.receipt, JSON.stringify({ ...receipt, status: "generation" }, null, 2) + "\n"); mkdirSync(dirname(paths.team), {recursive:true}); writeFileSync(paths.team,"## Testing Posture\nMethodology: tdd\n"); }],
  ["missing-plan", () => rmSync(paths.plan)],
  ["empty-instructions", () => writeFileSync(paths.instructions, " \n")],
  ["invalid-contract", () => writeFileSync(paths.plan, "# Plan\nNo contract\n")],
  ["unanswered", () => writeFileSync(paths.questions, readFileSync(paths.questions, "utf8").replace("[Answer]: Approve Plan", "[Answer]:"))],
  ["changed-fingerprint", () => writeFileSync(paths.questions, readFileSync(paths.questions, "utf8").replace(/^\[Approval Fingerprint\]: .*$/m, `[Approval Fingerprint]: sha256:${"f".repeat(64)}`))],
  ["changed-answered-document", () => writeFileSync(paths.questions, readFileSync(paths.questions, "utf8") + "\n")],
  ["changed-current-contract", () => { mkdirSync(dirname(paths.team), { recursive: true }); writeFileSync(paths.team, "## Testing Posture\nMethodology: tdd\n"); }],
];
try {
  for (const [id, mutate] of cases) {
    restore(); mutate();
    const current = posture.resolveTestingPosture(project);
    const sections = { org: "", team: id.endsWith("changed-current-contract") ? "Methodology: tdd\n" : "", project: "" };
    const context = { scope: current.scope, testStrategy: current.test_strategy, projectType: current.project_type };
    assert.deepEqual(posture.resolveTestingPostureFromSections(sections, context), current, "明示した規則入力は本家の実ファイル解決値と一致する");
    observations.push({ id, authority, context, sections, documents: { plan: existsSync(paths.plan) ? readFileSync(paths.plan, "utf8") : "", instructions: readFileSync(paths.instructions, "utf8"), questions: readFileSync(paths.questions, "utf8"), questions_file: questions }, receipt: existsSync(paths.receipt) ? JSON.parse(readFileSync(paths.receipt, "utf8")) : null, current_source: lib.workspaceSourceFingerprint(project), expected: posture.evaluateCodeGenerationApproval(project, { unit: null }) });
  }
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, capture_command: process.argv, normalization: [], observations }, null, 2) + "\n");
} finally { restore(); }
