#!/usr/bin/env bun
/** 本家2.7.1で、状態を手編集しないbugfix開始直後の配送を観測する。U2所有。 */
import assert from "node:assert/strict";
import { appendFileSync, cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { captureObservation } from "./capture-observation";
import { UPSTREAM, verifySource } from "./upstream-source";

const dist = process.argv[2];
const destination = process.argv[3];
const humanBetween = process.argv[4] === "--human-between";
const intervention = process.argv[4] ?? "";
assert(dist && destination, "usage: capture-selfhost-first-next.ts <verified-dist> <output.json>");
verifySource(dist);
const parent = mkdtempSync(join(tmpdir(), "aidlc-first-next-"));
const root = join(parent, "workspace");
const observations: Array<ReturnType<typeof captureObservation> & { id: string }> = [];
try {
  cpSync(dist, root, { recursive: true });
  mkdirSync(join(root, "src"), { recursive: true });
  writeFileSync(join(root, "src/base.ts"), "export const base = 1;\n");
  const run = (id: string, script: string, args: string[]) => {
    const observation = captureObservation({ root, argv: [process.execPath, join(root, ".claude/tools", script), ...args] });
    observations.push({ id, ...observation });
    assert.equal(observation.output.exit_code, 0, observation.output.stderr);
    assert.equal(observation.output.error, null);
    assert.equal(observation.output.signal, null);
    return observation.output;
  };
  run("intent-create/bugfix", "aidlc-utility.ts", ["intent-create", "--scope", "bugfix", "--label", "first-next", "--arguments", "Fix one small behavior"]);
  let directive = JSON.parse(run("next", "aidlc-orchestrate.ts", ["next"]).stdout);
  if (humanBetween) {
    const observation = captureObservation({ root, argv: [process.execPath, join(root, ".claude/hooks/aidlc-record-human-turn.ts")], stdin: JSON.stringify({ hook_event_name: "UserPromptSubmit", session_id: "steering-session", prompt: "Continue", cwd: root }) });
    observations.push({ id: "human-turn", ...observation });
    assert.equal(observation.output.exit_code, 0);
  }
  const statePath = () => join(root, "aidlc/spaces/default/intents", readFileSync(join(root, "aidlc/spaces/default/intents/active-intent"), "utf8").trim(), "aidlc-state.md");
  if (intervention === "--edit-state-between") appendFileSync(statePath(), "\n<!-- manual state edit -->\n");
  if (intervention === "--park-between") run("park", "aidlc-orchestrate.ts", ["park"]);
  if (intervention === "--noop-between") run("noop", "aidlc-orchestrate.ts", ["report", "--result", "completed", "--stage", "state-init"]);
  if (["--switch-intent-between", "--switch-shared-key-between"].includes(intervention)) {
    const original = readFileSync(statePath());
    const key = readFileSync(join(dirname(statePath()), ".aidlc-steering-token-key"));
    run("second-intent", "aidlc-utility.ts", ["intent-create", "--scope", "bugfix", "--label", "second", "--arguments", "Fix one small behavior"]);
    writeFileSync(statePath(), original);
    if (intervention === "--switch-shared-key-between") writeFileSync(join(dirname(statePath()), ".aidlc-steering-token-key"), key, { mode: 0o600 });
  }
  let part = 0;
  while (directive.kind === "load-steering") {
    assert(part++ < 20, "配送が有限に終わること");
    directive = JSON.parse(run(`continue/${part}`, "aidlc-orchestrate.ts", ["continue", directive.continue_token]).stdout);
  }
  const mustRefuse = ["--edit-state-between", "--park-between", "--switch-intent-between", "--switch-shared-key-between"].includes(intervention);
  assert.equal(directive.kind, mustRefuse ? "error" : "run-stage");
  if (!mustRefuse) assert.equal(directive.stage, "reverse-engineering");
  const referencePaths = [".claude/aidlc-common/conductor.md", ...["org.md", "team.md", "project.md", "phases/ideation.md", "phases/inception.md", "phases/construction.md", "phases/operation.md"].map(path => `aidlc/spaces/default/memory/${path}`)];
  const reference_files = Object.fromEntries(referencePaths.map(path => [path, readFileSync(join(root, path)).toString("base64")]));
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, capture_command: [process.execPath, import.meta.path, dist, destination, ...(intervention ? [intervention] : [])], tool_versions: { bun: Bun.version }, scenario: intervention || "bugfix-first-next-without-state-editing", fixture_kind: "isolated-workspace", reference_files, observations }, null, 2) + "\n");
} finally {
  rmSync(parent, { recursive: true, force: true });
}
