import { UPSTREAM, verifySource } from "./upstream-source";
import { captureObservation } from "./capture-observation";
import { cpSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import assert from "node:assert/strict";

export function captureLearnings(dist: string) {
  verifySource(dist);
  const observations: Array<ReturnType<typeof captureObservation> & { id: string }> = [];
  const root = mkdtempSync(join(tmpdir(), "aidlc-learnings-"));
  const run = (id: string, tool: string, args: string[]) => {
    const row = captureObservation({ root, argv: [process.execPath, join(root, ".claude/tools", tool), ...args] });
    assert.equal(row.output.error, null);
    assert.equal(row.output.signal, null);
    observations.push({ id: `learnings/${id}`, ...row });
    return row.output;
  };
  try {
    cpSync(dist, root, { recursive: true });
    const init = run("setup-intent", "aidlc-utility.ts", ["intent-create", "--scope", "bugfix", "--label", "learnings", "--arguments", "Capture learning behavior"]);
    assert.equal(init.exit_code, 0, init.stderr);
    const compile = run("setup-runtime", "aidlc-runtime.ts", ["compile"]);
    assert.equal(compile.exit_code, 0, compile.stderr);
    const surface = run("surface", "aidlc-learnings.ts", ["surface", "--slug", "requirements-analysis"]);
    assert.equal(surface.exit_code, 0, surface.stderr);
    const surfaced = JSON.parse(surface.stdout);
    const selection = { stage_slug: surfaced.stage_slug, space: surfaced.space, intent: surfaced.intent, selections: [] };
    writeFileSync(join(root, "selections.json"), JSON.stringify(selection));
    const empty = run("persist-empty", "aidlc-learnings.ts", ["persist", "--slug", "requirements-analysis", "--selections-json", "selections.json"]);
    assert.equal(empty.exit_code, 0, empty.stderr);
    const selected = { ...selection, selections: [{ candidate_id: "fixture-1", type: "learning", scope: "project", heading: "Corrections", text: "ALWAYS 採取用の検証結果を記録する。", source: "user_addition" }] };
    writeFileSync(join(root, "selections.json"), JSON.stringify(selected));
    const persisted = run("persist-one", "aidlc-learnings.ts", ["persist", "--slug", "requirements-analysis", "--selections-json", "selections.json"]);
    assert.equal(persisted.exit_code, 0, persisted.stderr);
    const repeated = run("persist-repeat", "aidlc-learnings.ts", ["persist", "--slug", "requirements-analysis", "--selections-json", "selections.json"]);
    assert.equal(repeated.exit_code, 0, repeated.stderr);
    const wrongStage = run("wrong-stage", "aidlc-learnings.ts", ["persist", "--slug", "code-generation", "--selections-json", "selections.json"]);
    assert.equal(wrongStage.exit_code, 1);
    writeFileSync(join(root, "selections.json"), JSON.stringify({ stage_slug: "requirements-analysis", selections: [] }));
    const malformed = run("malformed-selection", "aidlc-learnings.ts", ["persist", "--slug", "requirements-analysis", "--selections-json", "selections.json"]);
    assert.equal(malformed.exit_code, 1);
  } finally { rmSync(root, { recursive: true, force: true }); }
  return { source: UPSTREAM, fixture_kind: "synthetic-preconditions", observations };
}
