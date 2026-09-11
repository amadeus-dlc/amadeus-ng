#!/usr/bin/env bun
/** session bindingが壊れた共有cursorより優先される観測を保存する。 */
import assert from "node:assert/strict";
import { cpSync, mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { UPSTREAM, verifySource } from "./upstream-source";
import { captureObservation } from "./capture-observation";
const [dist, destination] = process.argv.slice(2);
assert(dist && destination); verifySource(dist);
const temporary = mkdtempSync(join(tmpdir(), "aidlc-session-selection-"));
const root = join(temporary, "workspace");
try {
  cpSync(dist, root, { recursive: true });
  const environment = { AIDLC_SESSION_OVERRIDE: "fixture-session" };
  const setup = captureObservation({ root, argv: [process.execPath, join(root, ".claude/tools/aidlc-utility.ts"), "intent-create", "--scope", "bugfix", "--label", "session-selection", "--arguments", "Observe the selected session"], environment });
  assert.equal(setup.output.exit_code, 0);
  writeFileSync(join(root, "aidlc/active-space"), "../escape\n");
  const probe = join(root, ".claude/tools/.session-selection-probe.ts");
  writeFileSync(probe, 'import { resolveWorkflowSelection } from "./aidlc-lib.ts";\nprocess.stdout.write(JSON.stringify(resolveWorkflowSelection(process.cwd())) + "\\n");\n');
  const observed = captureObservation({ root, argv: [process.execPath, probe], environment });
  assert.equal(observed.output.exit_code, 0);
  assert.equal(JSON.parse(observed.output.stdout).space, "default");
  assert.equal(JSON.parse(observed.output.stdout).sessionId, "fixture-session");
  assert.deepEqual(observed.changed_files, {});
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, JSON.stringify({ source: UPSTREAM, capture_method: "固定元を検証し、session override付き公開intent-createでbindingを作る。共有cursorだけを不正値へ置き、公開resolveWorkflowSelection関数を変更せず実行。", normalization: [], observations: [{ id: "binding-wins-over-invalid-shared-cursor", ...observed }] }, null, 2) + "\n");
} finally { rmSync(temporary, { recursive: true, force: true }); }
