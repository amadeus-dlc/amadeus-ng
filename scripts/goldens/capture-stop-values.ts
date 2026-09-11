#!/usr/bin/env bun
/** 固定本家の停止進捗署名と継続文言を、export追加だけの計測口から採取する。 */
import assert from "node:assert/strict";
import { cpSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { UPSTREAM, digest, verifySource } from "./upstream-source";
const [dist, destination] = process.argv.slice(2);
assert(dist && destination); verifySource(dist);
const temporary = mkdtempSync(join(tmpdir(), "aidlc-stop-values-"));
try {
  cpSync(dist, temporary, { recursive: true });
  const sourceFile = ".claude/hooks/aidlc-continue-workflow.ts";
  const original = readFileSync(join(temporary, sourceFile), "utf8");
  const measured = join(temporary, ".claude/hooks/.capture-stop-values.ts");
  writeFileSync(measured, original + "\nexport { progressSignature, continuationReason, blockStop };\n");
  const source = await import(pathToFileURL(measured).href);
  const state = "# State\n- **Current Stage**: code-generation\n- **Last Updated**: 2026-09-09T00:00:00Z\n- [-] code-generation — EXECUTE\n";
  const scenarios = [
    { id: "run", state, directive: { kind: "run-stage", stage: "code-generation" } },
    { id: "run-reverse-engineering", state: state.replaceAll("code-generation", "reverse-engineering"), directive: { kind: "run-stage", stage: "reverse-engineering" } },
    { id: "timestamp-only", state: state.replace("00:00:00", "01:00:00"), directive: { kind: "run-stage", stage: "code-generation" } },
    { id: "real-state-change", state: state.replace("[-]", "[?]"), directive: { kind: "run-stage", stage: "code-generation" } },
    { id: "steering", state, directive: { kind: "load-steering", stage: "code-generation", part: 1, parts: 2, continue_token: "opaque-token", rules_content: [{ path: "memory/team.md", text: "## Team\n原文を保つ。\n" }] } },
    { id: "next-part", state, directive: { kind: "load-steering", stage: "code-generation", part: 2, parts: 2, continue_token: "another-token", rules_content: [] } },
    { id: "unit", state, directive: { kind: "run-stage", stage: "code-generation", unit: "u2-workflow-authority" } },
    { id: "dispatch", state, directive: { kind: "dispatch-subagent", stage: "code-generation", worker: "aidlc-developer-agent", repo: "service" } },
    { id: "swarm", state, directive: { kind: "invoke-swarm", stage: "code-generation", units: ["u1", "u2"], wave: { id: "wave-1", parallel: true } } },
    { id: "irrelevant-message", state, directive: { kind: "run-stage", stage: "code-generation", message: "changed display only" } },
    { id: "no-stage", state: "# State\n", directive: { kind: "error" } },
  ];
  const observations = scenarios.map(({ id, state, directive }) => {
    const internal = { ...directive, continueToken: directive.continue_token, rulesContent: directive.rules_content };
    const reason = source.continuationReason(directive.kind, directive.stage ?? "", directive.continue_token, directive.rules_content);
    const lines: string[] = [];
    const originalLog = console.log;
    try {
      console.log = (line: string) => { assert.equal(typeof line, "string"); lines.push(line); };
      assert.equal(source.blockStop(reason), 0);
    } finally { console.log = originalLog; }
    return { id, state, directive, signature: source.progressSignature(state, internal), reason, stdout: lines.join("\n") + "\n" };
  });
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, source_file: sourceFile, source_sha256: digest(original), capture_method: "検証済み固定配布の複製にexport文のみ追記。関数本文・依存関数は無変更。公開directiveのcontinue_token/rules_contentを本家内部引数名へ写す。", normalization: [], observations }, null, 2) + "\n");
} finally { rmSync(temporary, { recursive: true, force: true }); }
