#!/usr/bin/env bun
/** 固定本家の再開待ちmarker検証と待機式を、原文のまま実行する。 */
import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { UPSTREAM, digest, verifySource } from "./upstream-source";

const [dist, destination] = process.argv.slice(2);
assert(dist && destination);
verifySource(dist);
const temporary = mkdtempSync(join(tmpdir(), "aidlc-stop-resume-"));
try {
  cpSync(dist, temporary, { recursive: true });
  const sourceFile = ".claude/tools/aidlc-lib.ts";
  const original = readFileSync(join(temporary, sourceFile), "utf8");
  const measured = join(temporary, ".claude/tools/.capture-stop-resume-wait.ts");
  writeFileSync(measured, original + "\nexport { parseActiveDirectiveMarker, stateContentSha256 };\n");
  const source = await import(pathToFileURL(measured).href);
  const expression = original.slice(original.indexOf("export function hasCurrentSharedResumeWait"))
    .match(/const waiting =([\s\S]*?);\s*return \{ marker, result: waiting/);
  assert(expression);
  const waiting = new Function("marker", "stateContent", "stateContentSha256", "getField", `return (${expression[1]});`);
  const state = "# State\n- **Construction Autonomy Mode**: gated\n";
  const stateHash = source.stateContentSha256(state);
  const base = {
    version: 2, stage: "code-generation", state_sha256: stateHash,
    project_sha256: "a".repeat(64), intent_uuid: null, state_present: true,
    owner_session: "sessionless:project", revision: 1, owner_epoch: 0, context_epoch: 0,
    event_sequence: 0, human_sequence: 0, engine_sequence: 0, conversation_sequence: 0, stop_count: 0,
    kind: "ask", delivery: "issued", needs_rehydrate: false,
    active_attempt: { command_kind: "next", command_sha256: "b".repeat(64), issued_state_sha256: stateHash, session_id: "", owner_epoch: 0, context_epoch: 0, status: "pending" },
    resume: { status: "waiting", issuing_stage: "code-generation", issuing_state_sha256: stateHash, issuing_session: "", issuing_intent_uuid: null },
  };
  const scenarios: Array<{ id: string; marker: string; state: string }> = [];
  const add = (id: string, value: unknown, currentState = state) => scenarios.push({ id, marker: JSON.stringify(value), state: currentState });
  const change = (id: string, path: string, value: unknown) => {
    const marker = structuredClone(base) as Record<string, unknown>;
    const keys = path.split(".");
    const key = keys.pop()!;
    let target = marker;
    for (const part of keys) target = target[part] as Record<string, unknown>;
    if (value === undefined) delete target[key]; else target[key] = value;
    add(id, marker);
  };
  add("valid-shared-wait", base);
  add("changed-state", base, state + "changed\n");
  for (const [id, value] of [["null", null], ["array", []], ["string", "marker"], ["empty-object", {}]] as const) add(id, value);
  scenarios.push({ id: "broken-json", marker: "{", state });
  for (const key of Object.keys(base)) change(`missing-${key}`, key, undefined);
  for (const key of ["revision", "owner_epoch", "context_epoch", "event_sequence", "human_sequence", "engine_sequence", "conversation_sequence", "stop_count"]) change(`negative-${key}`, key, -1);
  change("fractional-revision", "revision", 0.5);
  change("integer-float-revision", "revision", 1.0);
  change("large-integer-revision", "revision", 1e30);
  change("string-revision", "revision", "1");
  change("v1", "version", 1);
  change("v3", "version", 3);
  change("string-version", "version", "2");
  change("stage-trim", "stage", "\uFEFFcode-generation\n");
  for (const value of ["", "1-stage", "Stage", "stage_name", "a/b"]) change(`stage-${JSON.stringify(value)}`, "stage", value);
  change("state-hash-uppercase", "state_sha256", stateHash.toUpperCase());
  change("state-hash-short", "state_sha256", "a".repeat(63));
  change("state-hash-array", "state_sha256", [stateHash]);
  change("state-hash-final-newline", "state_sha256", stateHash + "\n");
  change("project-hash-array-coercion", "project_sha256", ["a".repeat(64)]);
  change("project-hash-short", "project_sha256", "a".repeat(63));
  change("intent-string", "intent_uuid", "");
  change("intent-number", "intent_uuid", 1);
  change("present-false", "state_present", false);
  change("present-number", "state_present", 1);
  change("owner-not-shared", "owner_session", "session-1");
  change("owner-empty", "owner_session", "");
  change("rehydrate-true", "needs_rehydrate", true);
  change("rehydrate-number", "needs_rehydrate", 0);
  change("unit-trim", "unit", " Unit.One ");
  change("unit-opaque-nonempty", "unit", "../opaque");
  change("unit-empty", "unit", " ");
  change("unit-null", "unit", null);
  for (const value of [[], ["Unit.One"], ["9-unit"], ["a".repeat(64)], ["a".repeat(65)], ["../unit"], [null], "unit"]) change(`units-${JSON.stringify(value)}`, "units", value);
  for (const value of ["codex", "Claude_1", "", "-claude", null]) change(`cursor-${JSON.stringify(value)}`, "cursor_harness", value);
  for (const value of ["a".repeat(40), "a".repeat(64), "unbindable", "A".repeat(64), null]) change(`source-${JSON.stringify(value)}`, "code_generation_source_sha256", value);
  change("authority-zero", "code_generation_authority_revision", 0);
  change("authority-negative", "code_generation_authority_revision", -1);
  change("kind-run", "kind", "run-stage");
  change("kind-unknown", "kind", "unknown");
  change("delivery-array-coercion", "delivery", ["issued"]);
  change("delivery-unknown", "delivery", "unknown");
  for (const key of Object.keys(base.active_attempt)) change(`missing-attempt-${key}`, `active_attempt.${key}`, undefined);
  change("attempt-id-string", "active_attempt.id", "");
  change("attempt-id-number", "active_attempt.id", 1);
  change("attempt-command-array", "active_attempt.command_kind", ["next"]);
  change("attempt-command-unknown", "active_attempt.command_kind", "unknown");
  change("attempt-shared-false", "active_attempt.shared_attempt", false);
  change("attempt-shared-number", "active_attempt.shared_attempt", 0);
  for (const key of ["claim_revision", "result_revision", "resume_gate_revision"]) {
    change(`attempt-${key}-valid`, `active_attempt.${key}`, 0);
    change(`attempt-${key}-invalid`, `active_attempt.${key}`, -1);
  }
  for (const key of ["cursor_input_sha256", "result_sha256"]) {
    change(`attempt-${key}-valid`, `active_attempt.${key}`, "a".repeat(64));
    change(`attempt-${key}-invalid`, `active_attempt.${key}`, "wrong");
  }
  for (const key of Object.keys(base.resume)) change(`missing-resume-${key}`, `resume.${key}`, undefined);
  change("resume-selected", "resume.status", "selected");
  change("resume-array-status", "resume.status", ["waiting"]);
  change("resume-state-hash-short", "resume.issuing_state_sha256", "wrong");
  for (const token of ["", "token", "あ".repeat(5461), "あ".repeat(5462)]) {
    add(`token-${token.length}`, { ...base, continue_token: token, continue_token_sha256: digest(token) });
  }
  add("token-wrong-digest", { ...base, continue_token: "token", continue_token_sha256: "0".repeat(64) });
  add("token-null", { ...base, continue_token: null, continue_token_sha256: digest("") });
  const observations = scenarios.map(item => {
    let parsed: unknown = null;
    try { parsed = source.parseActiveDirectiveMarker(JSON.parse(item.marker)); } catch {}
    return { ...item, state_sha256: source.stateContentSha256(item.state), waiting: waiting(parsed, item.state, source.stateContentSha256, source.getField) === true };
  });
  assert.equal(new Set(observations.map(row => row.id)).size, observations.length);
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, source_file: sourceFile, source_sha256: digest(original), predicate_sha256: digest(expression[1]), capture_method: "固定本家parserとstateContentSha256はexport追加のみ。hasCurrentSharedResumeWaitのconst waiting式を原文抽出して実行。状態は非自律に固定し、IO/ロックはこの検査の外。", normalization: [], observations }, null, 2) + "\n");
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
