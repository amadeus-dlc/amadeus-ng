#!/usr/bin/env bun
/** SessionStartの文面式だけを固定元から抜き出し、判断済みの表示材料で実行する。 */
import assert from "node:assert/strict";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { UPSTREAM, digest, verifySource } from "./upstream-source";

const [dist, destination] = process.argv.slice(2);
assert(dist && destination);
verifySource(dist);
const source = readFileSync(join(dist, ".claude/hooks/aidlc-session-start.ts"), "utf8");
const expression = source.match(/const context = (`[\s\S]*?`);\n\n\/\/ Output additionalContext/)?.[1];
const recovery = source.match(/const recovery = existsSync\(recoveryFile\)\s*\? (".*")\s*:/)?.[1];
const drift = source.match(/^    driftNote =\s*([\s\S]*?);\n\s*}/m)?.[1];
const rebind = source.match(/rebindOffer =\s*(`INTENT REBIND OFFER:[\s\S]*?);\n\s*writeSessionRebindOffer/)?.[1];
const cold = source.match(/additionalContext:\s*(`AIDLC Runtime Session: \$\{sessionId\}\\n`\s*\+\s*"Use[^\n]*")/)?.[1];
const probe = source.match(/additionalContext:\s*(`AIDLC Runtime Session: \$\{sessionId\}\\n\$\{rebindOffer\}`)/)?.[1];
assert(expression && recovery && drift && rebind && cold && probe);
const render = new Function("scope", "sessionId", "phase", "stage", "status", "agent", "last", "next", "rebindOffer", "unitLine", "recovery", "driftNote", `return ${expression};`);
const recoveryText = new Function(`return ${recovery};`)();
const driftText = new Function("uncompiledStages", "harnessDir", `return ${drift};`);
const rebindText = new Function("was", "liveSlug", "switchInstruction", `return ${rebind};`);
const coldText = new Function("sessionId", `return ${cold};`);
const probeText = new Function("sessionId", "rebindOffer", `return ${probe};`);
const base = { scope: "bugfix", session: "session-a", phase: "CONSTRUCTION", stage: "code-generation", status: "Running", agent: "aidlc-developer-agent", last: "requirements-analysis", next: "Execute Code Generation", rebind_offer: "", unit_line: "", recovery: false, uncompiled_stages: [] as string[] };
const offer = rebindText({ slug: "original" }, "another", "run `/aidlc intent original`");
const cross = rebindText({ slug: "original" }, "(none)", "first run `/aidlc space other`; after it completes, run `/aidlc intent original`");
const observations: unknown[] = [];
for (const [id, change] of [
  ["active", {}], ["no-session", { session: "" }],
  ["empty-fields", { scope: "", phase: "", stage: "", status: "", agent: "", last: "", next: "" }],
  ["unicode", { scope: "日本語", session: "session-😀", next: '行1\n行2\t"引用"\\末尾' }],
  ["recovery", { recovery: true }], ["rebind", { rebind_offer: offer }],
  ["drift", { uncompiled_stages: ["inception/new-stage.md", "construction/追加.md"] }],
  ["all-notices", { rebind_offer: cross, recovery: true, unit_line: "Active Unit: u2 (paused; reason: review; next: continue)\n", uncompiled_stages: ["construction/new.md"] }],
] as const) {
  const input = { ...base, ...change };
  const context = render(input.scope, input.session, input.phase, input.stage, input.status, input.agent, input.last, input.next, input.rebind_offer, input.unit_line, input.recovery ? recoveryText : "", input.uncompiled_stages.length ? driftText(input.uncompiled_stages, () => ".claude") : "");
  observations.push({ id, kind: "workflow", input, stdout: JSON.stringify({ additionalContext: context }) + "\n" });
}
for (const session of ["session-a", '日本語-😀"']) {
  observations.push({ id: `cold/${session}`, kind: "runtime", input: { session }, stdout: JSON.stringify({ additionalContext: coldText(session) }) + "\n" });
}
observations.push({ id: "probe", kind: "probe", input: { session: "session-a", offer }, stdout: JSON.stringify({ additionalContext: probeText("session-a", offer) }) + "\n" });
mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, JSON.stringify({ source: UPSTREAM, hook_sha256: digest(source), capture_method: "固定元のcontext/recovery/drift/rebind/cold/probeの式を原文のまま実行。表示材料は判断済み。ファイルIO・session帰属・発火判定を検証する採取ではない。", normalization: [], rebind_cases: [{ was: "original", live: "another", instruction: "run `/aidlc intent original`", expected: offer }, { was: "original", live: "(none)", instruction: "first run `/aidlc space other`; after it completes, run `/aidlc intent original`", expected: cross }], observations }, null, 2) + "\n");
