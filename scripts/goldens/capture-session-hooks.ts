#!/usr/bin/env bun
/** 固定本家のsession/subagent/PreCompactを実プロセスで観測する。 */
import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { UPSTREAM, verifySource } from "./upstream-source";
import { captureObservation } from "./capture-observation";

const [dist, destination] = process.argv.slice(2);
assert(dist && destination);
verifySource(dist);
const cases = [
  { id: "cold/startup", active: false, hook: "session-start", input: '{"source":"startup","session_id":"session-a"}' },
  { id: "cold/end", active: false, hook: "session-end", input: '{"session_id":"session-a","reason":"exit"}' },
  { id: "cold/subagent", active: false, hook: "log-subagent", input: '{"agent_type":"aidlc-developer-agent","session_id":"session-a"}' },
  { id: "cold/compact", active: false, hook: "validate-state", input: '{"session_id":"session-a"}' },
  ...["startup", "resume", "clear", "compact", "unknown"].map(source => ({ id: `active/${source}`, active: true, hook: "session-start", input: JSON.stringify({ source, session_id: "session-a" }) })),
  { id: "active/malformed-start", active: true, hook: "session-start", input: "{" },
  { id: "active/empty-start", active: true, hook: "session-start", input: "" },
  { id: "active/null-start", active: true, hook: "session-start", input: "null" },
  { id: "active/unbound-end", active: true, hook: "session-end", input: '{"session_id":"session-a","reason":"exit"}' },
  { id: "active/bound-end", active: true, bind: true, hook: "session-end", input: '{"session_id":"session-a","reason":"exit"}' },
  { id: "active/anonymous-end", active: true, hook: "session-end", input: "{}" },
  { id: "active/subagent", active: true, hook: "log-subagent", input: '{"session_id":"session-a","agent_type":"aidlc-developer-agent","agent_id":"worker-a","last_assistant_message":"Completed the delegated work."}' },
  { id: "active/subagent-missing-fields", active: true, hook: "log-subagent", input: "{}" },
  { id: "active/subagent-malformed", active: true, hook: "log-subagent", input: "{" },
  { id: "active/subagent-long-message", active: true, hook: "log-subagent", input: JSON.stringify({ agent_type: "aidlc-developer-agent", last_assistant_message: "あ".repeat(199) + "😀after" }) },
  { id: "completed/subagent", active: true, completed: true, hook: "log-subagent", input: '{"agent_type":"aidlc-developer-agent"}' },
  { id: "active/compact-valid", active: true, hook: "validate-state", input: '{"session_id":"session-a"}' },
  { id: "active/compact-invalid", active: true, invalid: true, hook: "validate-state", input: "{}" },
];
const observations: unknown[] = [];
for (const scenario of cases) {
  const temporary = mkdtempSync(join(tmpdir(), "aidlc-session-hooks-"));
  const root = join(temporary, "workspace");
  try {
    cpSync(dist, root, { recursive: true });
    mkdirSync(join(root, "src"));
    writeFileSync(join(root, "src/lib.rs"), "pub fn smoke() {}\n");
    if (scenario.active) {
      const setup = captureObservation({ root, argv: [process.execPath, join(root, ".claude/tools/aidlc-utility.ts"), "intent-create", "--scope", "bugfix", "--label", "session-hooks", "--arguments", "Verify session hooks"] });
      assert.equal(setup.output.exit_code, 0, setup.output.stderr);
      const record = join(root, "aidlc/spaces/default/intents", readFileSync(join(root, "aidlc/spaces/default/intents/active-intent"), "utf8").trim());
      if ("bind" in scenario && scenario.bind) {
        const binding = captureObservation({ root, argv: [process.execPath, join(root, ".claude/hooks/aidlc-session-start.ts")], stdin: '{"source":"startup","session_id":"session-a"}' });
        assert.equal(binding.output.exit_code, 0, binding.output.stderr);
      }
      if ("completed" in scenario && scenario.completed) {
        const path = join(record, "aidlc-state.md");
        writeFileSync(path, readFileSync(path, "utf8").replace("**Status**: Running", "**Status**: Completed"));
      }
      if ("invalid" in scenario && scenario.invalid) {
        const path = join(record, "aidlc-state.md");
        writeFileSync(path, readFileSync(path, "utf8").replace("## Stage Progress", "## Missing Progress").replace("## Current Status", "## Missing Status"));
      }
    }
    const observed = captureObservation({ root, argv: [process.execPath, join(root, `.claude/hooks/aidlc-${scenario.hook}.ts`)], stdin: scenario.input, environment: { AIDLC_DISABLE_USAGE_TRACKING: "1" } });
    assert.equal(observed.output.exit_code, 0, `${scenario.id}: ${observed.output.stderr}`);
    observations.push({ id: scenario.id, ...observed });
  } finally { rmSync(temporary, { recursive: true, force: true }); }
}
mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, JSON.stringify({ source: UPSTREAM, capture_method: "固定配布全文検証後、実CLIでBrownfield bugfixを作成し実フックを別プロセスで呼ぶ。Completed/欠落セクションは明示fixture。出力・前後ファイルは無加工。usage ledgerは無効化し、transcript有効経路の証拠とはしない。", normalization: [], observations }, null, 2) + "\n");
