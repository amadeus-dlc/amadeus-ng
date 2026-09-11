#!/usr/bin/env bun
/** 固定本家のClaude会話履歴判定を、関数本文を変えずに採取する。 */
import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { UPSTREAM, digest, verifySource } from "./upstream-source";

const [dist, destination] = process.argv.slice(2);
assert(dist && destination);
verifySource(dist);
const temporary = mkdtempSync(join(tmpdir(), "aidlc-stop-transcript-"));
try {
  cpSync(dist, temporary, { recursive: true });
  const sourceFile = ".claude/hooks/aidlc-continue-workflow.ts";
  const original = readFileSync(join(temporary, sourceFile), "utf8");
  const measured = join(temporary, ".claude/hooks/.capture-stop-transcript.ts");
  writeFileSync(measured, original + "\nexport { transcriptIsConversational };\n");
  const source = await import(pathToFileURL(measured).href);
  const user = (content: unknown, extra = {}) => ({ type: "user", message: { role: "user", content }, ...extra });
  const assistant = (content: unknown) => ({ type: "assistant", message: { role: "assistant", content } });
  const tool = (command: unknown, name: unknown = "Bash") => assistant([{ type: "tool_use", name, input: { command } }]);
  const human = user("この状態を説明して");
  const engine = tool("bun .claude/tools/aidlc-orchestrate.ts next");
  const scenarios: Array<{ id: string; transcript: string }> = [];
  const add = (id: string, entries: unknown[]) => scenarios.push({ id, transcript: entries.map(entry => JSON.stringify(entry)).join("\n") + "\n" });
  scenarios.push({ id: "empty", transcript: "" }, { id: "broken-lines", transcript: "{\nnot-json\n" });
  add("scalars", [null, [], 3, true, "text"]);
  add("assistant-only", [assistant([{ type: "text", text: "回答" }])]);
  add("human-text", [human]);
  add("human-empty-text", [user("")]);
  add("human-text-block", [user([{ type: "text", text: "質問" }])]);
  add("human-null-text-block", [user([{ type: "text", text: null }])]);
  add("image-only", [user([{ type: "image", source: {} }])]);
  add("meta-only", [user("prompt", { isMeta: true })]);
  add("meta-number-is-not-true", [user("prompt", { isMeta: 1 })]);
  add("type-role-mismatch", [{ type: "user", message: { role: "assistant", content: "質問" } }]);
  add("engine-after-human", [human, engine]);
  add("new-human-after-engine", [human, engine, user("別の質問")]);
  add("tool-result-is-not-human", [human, engine, user([{ type: "tool_result", content: "result" }])]);
  add("mixed-tool-result-is-not-human", [human, engine, user([{ type: "text", text: "質問" }, { type: "tool_result", content: "result" }])]);
  add("meta-after-engine", [human, engine, user("追記", { isMeta: true })]);
  add("wrapped-feedback", [human, engine, user("Stop hook feedback: continue")]);
  add("raw-feedback", [human, engine, user("The AIDLC workflow has a pending step; follow the workflow loop")]);
  add("raw-without-loop-phrase", [human, engine, user("The AIDLC workflow has a pending step")]);
  add("bom-feedback", [human, engine, user("\uFEFFStop hook feedback: continue")]);
  add("nel-is-not-js-whitespace", [human, engine, user("\u0085Stop hook feedback: continue")]);
  add("split-feedback-blocks", [human, engine, user([{ type: "text", text: "Stop hook " }, { type: "text", text: "feedback: continue" }])]);
  add("assistant-text-is-not-tool", [human, assistant([{ type: "text", text: "aidlc next" }])]);
  add("assistant-string-is-not-tool", [human, assistant("aidlc next")]);
  add("assistant-wrong-role", [human, { type: "assistant", message: { role: "user", content: [{ type: "tool_use", name: "Bash", input: { command: "aidlc next" } }] } }]);
  add("other-harness-shape", [human, { type: "response_item", payload: { type: "function_call", name: "Bash", arguments: "aidlc next" } }]);
  scenarios.push({ id: "partial-tail-keeps-engine", transcript: [human, engine].map(entry => JSON.stringify(entry)).join("\n") + "\n{\"type\":" });
  add("lone-surrogate-human", [user("\ud800")]);
  add("lone-surrogate-in-unused-field", [human, { ...engine, unused: "\udfff" }]);
  scenarios.push({ id: "overflow-in-unused-field", transcript: JSON.stringify(human) + '\n{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","name":"Bash","input":{"command":"aidlc next"}}]},"unused":1e9999}\n' });
  scenarios.push({ id: "invalid-line-after-human", transcript: JSON.stringify(human) + '\n{"unfinished":\n' });
  const commands = [
    "git status", "cat aidlc-state.md", "aidlc-log decision", "aidlc-utility intent-create",
    "aidlc-orchestrate.ts next", "aidlc-orchestrate.ts report", "aidlc-orchestrate.ts park",
    "aidlc-orchestrate.ts next --status", "aidlc-orchestrate.ts report --status", "aidlc-orchestrate.ts --help",
    "aidlc-state.ts get", "aidlc-state.ts approve", "aidlc-state.ts checkbox", "aidlc-state.ts unknown",
    "aidlc-jump.ts --status", "aidlc-jump.ts execute", "aidlc-unit.ts status", "aidlc-unit.ts start --status",
    "aidlc next", "aidlc next --doctor", "aidlc report --status", "aidlc park --help",
    "aidlc orchestrate next", "aidlc orchestrate next --help", "aidlc orchestrate park", "aidlc orchestrate unknown",
    "aidlc state init", "aidlc state show", "aidlc state set-status", "aidlc unit status", "aidlc unit start --status",
    "aidlc jump --version", "aidlc jump execute", "aidlc bolt start", "aidlc swarm --help",
    "aidlc next --status && aidlc report", "aidlc next --status; aidlc park", "aidlc next --status || aidlc state approve",
    "aidlc-orchestrate & report", "echo 'aidlc next'", "aidlc next --status-extra", "aidlc next --status_report",
    "üaidlc next", "xaidlc next", "xaidlc-orchestrate next", "aidlc\uFEFFnext", "aidlc\u0085next", "aidlc\nnext",
  ];
  commands.forEach((command, index) => add(`command-${index + 1}`, [human, tool(command)]));
  add("shell-case-insensitive", [human, tool("aidlc next", "sHeLl")]);
  add("named-engine-tool", [human, tool(null, "aidlc-orchestrate next")]);
  add("other-tool-command-ignored", [human, tool("aidlc next", "Read")]);
  add("single-command-array", [human, tool(["aidlc next"])]);
  add("split-command-array", [human, tool(["aidlc", "next"])]);
  add("null-command", [human, tool(null)]);
  add("multiple-blocks", [human, assistant([{ type: "text", text: "作業" }, { type: "tool_use", name: "Bash", input: { command: "aidlc next" } }])]);
  const transcript = join(temporary, "claude-transcript.jsonl");
  const observations = scenarios.map(scenario => {
    writeFileSync(transcript, scenario.transcript);
    return { ...scenario, conversational: source.transcriptIsConversational(transcript, "claude") };
  });
  assert.equal(new Set(observations.map(row => row.id)).size, observations.length);
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, source_file: sourceFile, source_sha256: digest(original), capture_method: "固定配布の複製にexport文のみ追加。関数本文・isEngineToolCall等の依存は無変更。Claude形式だけを採取。", normalization: [], observations }, null, 2) + "\n");
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
