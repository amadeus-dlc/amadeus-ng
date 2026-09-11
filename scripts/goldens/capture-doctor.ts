import { UPSTREAM, verifySource } from "./upstream-source";
import { captureObservation } from "./capture-observation";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import assert from "node:assert/strict";

export function captureDoctor(dist: string) {
  verifySource(dist);
  const observations: Array<ReturnType<typeof captureObservation> & { id: string; fixture_changes: Record<string, string | null>; fixture_directories: string[]; fixture_lock: string | null }> = [];
  const scenarios = [
    "cold", "initialized", "missing-bun", "missing-hook", "disabled-hooks",
    "missing-settings", "no-hooks", "invalid-settings", "managed-only",
    "version-missing", "version-past", "version-future",
    "heartbeat-boundary", "heartbeat-stale", "heartbeat-unreadable",
    "missing-stage", "invalid-stage", "invalid-reference", "cyclic-graph", "audit-locked",
  ];
  for (const id of scenarios) {
    const root = mkdtempSync(join(tmpdir(), "aidlc-doctor-"));
    let unlock: (() => void) | undefined;
    try {
      cpSync(dist, root, { recursive: true });
      const fixture_changes: Record<string, string | null> = {};
      const fixture_directories: string[] = [];
      const change = (path: string, contents: string | null) => {
        if (contents === null) rmSync(join(root, path));
        else { mkdirSync(dirname(join(root, path)), { recursive: true }); writeFileSync(join(root, path), contents); }
        fixture_changes[path] = contents === null ? null : Buffer.from(contents).toString("base64");
      };
      if (id === "missing-hook") change(".claude/hooks/aidlc-record-human-turn.ts", null);
      if (id === "missing-settings") change(".claude/settings.json", null);
      if (id === "no-hooks") change(".claude/settings.json", "{}\n");
      if (id === "invalid-settings") change(".claude/settings.json", '{"hooks":');
      if (id === "managed-only") change("aidlc/.capture-managed.json", '{"allowManagedHooksOnly":true}\n');
      if (id === "disabled-hooks") {
        const settings = JSON.parse(readFileSync(join(root, ".claude/settings.json"), "utf8"));
        change(".claude/settings.json", JSON.stringify({ ...settings, disableAllHooks: true }));
      }
      const stagePath = ".claude/aidlc-common/stages/construction/code-generation.md";
      if (id === "missing-stage") change(stagePath, null);
      if (id === "invalid-stage") change(stagePath, readFileSync(join(root, stagePath), "utf8").replace(/^mode:.*$/m, "mode: invalid-mode"));
      if (id === "invalid-reference" || id === "cyclic-graph") {
        const graphPath = ".claude/tools/data/stage-graph.json";
        const graph = JSON.parse(readFileSync(join(root, graphPath), "utf8"));
        const target = graph.find((stage: { slug: string }) => stage.slug === "code-generation");
        assert(target);
        target.requires_stage.push(id === "cyclic-graph" ? "code-generation" : "not-a-stage");
        change(graphPath, JSON.stringify(graph));
      }
      const command = [process.execPath, join(root, ".claude/tools/aidlc-utility.ts")];
      if (id === "initialized" || id === "audit-locked" || id.startsWith("version-") || id.startsWith("heartbeat-")) {
        const init = captureObservation({ root, argv: [...command, "intent-create", "--scope", "bugfix", "--label", "doctor", "--arguments", "Diagnose this fixture"] });
        assert.equal(init.output.exit_code, 0, init.output.stderr);
        observations.push({ id: `doctor/setup-${id}`, fixture_changes: {}, fixture_directories: [], fixture_lock: null, ...init });
        const records = "aidlc/spaces/default/intents";
        const entries = readdirSync(join(root, records), { withFileTypes: true }).filter(entry => entry.isDirectory());
        assert.equal(entries.length, 1);
        const record = join(records, entries[0].name);
        if (id === "audit-locked") {
          // 固定本家の公開ロックAPIで競合を作る。監査や承認行は手書きしない。
          const locks = require(join(root, ".claude/tools/aidlc-lib.ts")) as {
            acquireAuditLock(root: string, retries: number, delay: number): boolean;
            releaseAuditLock(root: string): void;
          };
          assert(locks.acquireAuditLock(root, 1, 1), "採取用の監査ロックを取得できない");
          unlock = () => locks.releaseAuditLock(root);
        }
        if (id.startsWith("version-")) {
          const state = join(record, "aidlc-state.md");
          const content = readFileSync(join(root, state), "utf8");
          const replacement = id === "version-missing" ? "" : `- **State Version**: ${id === "version-past" ? 7 : 9}`;
          change(state, content.replace(/^- \*\*State Version\*\*:.*$/m, replacement));
        }
        if (id === "heartbeat-unreadable") {
          const path = join(record, ".aidlc-hooks-health/unreadable.last");
          mkdirSync(join(root, path), { recursive: true });
          fixture_directories.push(path);
        } else if (id.startsWith("heartbeat-")) {
          const auditDir = join(root, record, "audit");
          const audits = readdirSync(auditDir).filter(name => name.endsWith(".md")).map(name => readFileSync(join(auditDir, name), "utf8")).join("\n");
          const dates = audits.split(/^## /m).filter(block => /\*\*Event\*\*:\s*(STAGE_|GATE_)/.test(block)).map(block => Date.parse(block.match(/^\*\*Timestamp\*\*:\s*(.+)$/m)?.[1] ?? ""));
          assert(dates.length > 0 && dates.every(Number.isFinite));
          const timestamp = new Date(Math.max(...dates) - (id === "heartbeat-boundary" ? 300000 : 300001)).toISOString();
          change(join(record, ".aidlc-hooks-health/fixture.last"), timestamp);
        }
      }
      const environment = {
        AIDLC_MANAGED_SETTINGS_PATH: join(root, "aidlc/.capture-managed.json"),
        ...(id === "missing-bun" ? { PATH: join(root, "no-bun") } : {}),
        // 親が公開APIで保持したロックと、子CLIの一時領域を同じにする。
        ...(id === "audit-locked" ? { TMPDIR: tmpdir() } : {}),
      };
      const observation = captureObservation({ root, argv: [...command, "doctor"], environment });
      assert.equal(observation.output.error, null);
      assert.equal(observation.output.signal, null);
      observations.push({ id: `doctor/${id}`, fixture_changes, fixture_directories, fixture_lock: id === "audit-locked" ? "parent holds upstream acquireAuditLock for this intent" : null, ...observation });
    } finally {
      unlock?.();
      rmSync(root, { recursive: true, force: true });
    }
  }
  return { source: UPSTREAM, fixture_kind: "synthetic-preconditions", observations };
}
