#!/usr/bin/env bun
/** 固定本家write-audit-logの配置・heartbeat・drop追記を、実フック呼出しで観測する。 */
import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { UPSTREAM, verifySource } from "./upstream-source";
import { captureObservation } from "./capture-observation";
const [dist, destination] = process.argv.slice(2);
assert(dist && destination); verifySource(dist);
const observations: unknown[] = [];
for (const scenario of ["cold-malformed", "cold-null", "active-malformed", "active-outside", "active-missing-file", "audit-directory", "heartbeat-directory"]) {
  const parent = mkdtempSync(join(tmpdir(), "aidlc-write-health-"));
  const root = join(parent, "project");
  try {
    cpSync(dist, root, { recursive: true });
    let record = "aidlc/spaces/default/intents";
    if (!scenario.startsWith("cold-")) {
      const setup = captureObservation({ root, argv: [process.execPath, join(root, ".claude/tools/aidlc-utility.ts"), "intent-create", "--scope", "bugfix", "--label", "hook-health", "--arguments", "Observe hook health"] });
      assert.equal(setup.output.exit_code, 0, setup.output.stderr);
      observations.push({ id: `${scenario}/setup`, fixture_directories: [], ...setup });
      const records = readdirSync(join(root, record), { withFileTypes: true }).filter(entry => entry.isDirectory() && !entry.name.startsWith("."));
      assert.equal(records.length, 1);
      record = join(record, records[0].name);
    }
    const directories: string[] = [];
    if (scenario === "audit-directory") {
      const audits = readdirSync(join(root, record, "audit")).filter(name => name.endsWith(".md"));
      assert.equal(audits.length, 1);
      const audit = join(record, "audit", audits[0]);
      rmSync(join(root, audit)); mkdirSync(join(root, audit)); directories.push(audit);
    }
    if (scenario === "heartbeat-directory") {
      const heartbeat = join(record, ".aidlc-hooks-health/write-audit-log.last");
      mkdirSync(join(root, heartbeat), { recursive: true }); directories.push(heartbeat);
    }
    const stdin = scenario.endsWith("malformed") ? "{" : scenario === "cold-null" ? "null" : JSON.stringify({ hook_event_name: "PostToolUse", tool_name: "Write", tool_input: { file_path: join(root, scenario === "active-outside" ? "README.md" : join(record, "missing-artifact.md")) } });
    for (let invocation = 1; invocation <= (scenario === "audit-directory" ? 2 : 1); invocation++) {
      const observation = captureObservation({ root, argv: [process.execPath, join(root, ".claude/hooks/aidlc-write-audit-log.ts")], stdin });
      const healthFiles = Object.keys(observation.changed_files).filter(path => path.includes(".aidlc-hooks-health/"));
      if (scenario !== "heartbeat-directory") assert.equal(observation.output.exit_code, 0, observation.output.stderr);
      if (scenario === "audit-directory") assert(healthFiles.some(path => path.endsWith(".drops")));
      observations.push({ id: `${scenario}/${invocation}`, record, fixture_directories: directories, ...observation });
    }
  } finally { rmSync(parent, { recursive: true, force: true }); }
}
mkdirSync(dirname(destination), { recursive: true });
writeFileSync(destination, JSON.stringify({ source: UPSTREAM, capture_method: "固定配布を同じbasename projectにコピーし、実CLIを別プロセスで起動。初期化も本家intent-create。監査書込障害は現在シャードを同名ディレクトリに置換。既存結果を流用せず前後全文を保存。", normalization: ["比較時のみ入力rootと時刻を契約C1の範囲で対応付ける。保存バイトは無変更。"], observations }, null, 2) + "\n");
